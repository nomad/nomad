use core::{iter, str};
use std::{env, fs, path, process};

use abs_path::{AbsPath, AbsPathBuf, NodeName, NodeNameBuf, node};
use anyhow::{Context, anyhow};
use cargo_metadata::TargetKind;

use crate::WORKSPACE_ROOT;
use crate::neovim::CARGO_TOML_META;

#[derive(Debug, Clone, clap::Args)]
pub(crate) struct BuildArgs {
    /// Build the plugin for the latest nightly version of Neovim.
    #[clap(long)]
    nightly: bool,

    /// Build the plugin in release mode.
    #[clap(long, short)]
    release: bool,

    /// The target triple to build the plugin for.
    #[clap(long)]
    target: Option<String>,

    /// The absolute path to the directory under which to place the build
    /// artifacts.
    #[clap(long, default_value_t = WORKSPACE_ROOT.to_owned())]
    out_dir: AbsPathBuf,
}

pub(crate) fn build(args: BuildArgs) -> anyhow::Result<()> {
    fs::create_dir_all(&args.out_dir)?;

    let artifact_dir = args.out_dir.clone().join(node!("lua"));

    // Setting the artifact directory is still unstable.
    let artifact_dir_args = ["-Zunstable-options", "--artifact-dir"]
        .into_iter()
        .chain(iter::once(artifact_dir.as_str()));

    let target_args =
        args.target.as_deref().map(|target| ["--target", target]);

    let exit_status = process::Command::new("cargo")
        .arg("build")
        .args(artifact_dir_args)
        .args(["--package", &CARGO_TOML_META.name])
        .args(args.nightly.then_some("--features=neovim-nightly"))
        .args(args.release.then_some("--release"))
        .args(target_args.as_ref().map(|args| &args[..]).unwrap_or_default())
        .status()?;

    if !exit_status.success() {
        process::exit(exit_status.code().unwrap_or(1));
    }

    fix_library_name(&artifact_dir)?;

    let dst = artifact_dir.join(node!("nomad"));
    if !fs::exists(&dst)? {
        let src = WORKSPACE_ROOT.join(node!("lua")).join(node!("nomad"));
        copy_dir(&src, &dst)?;
    }

    Ok(())
}

#[allow(clippy::unwrap_used)]
fn fix_library_name(artifact_dir: &AbsPath) -> anyhow::Result<()> {
    let package_meta = &CARGO_TOML_META;

    let mut cdylib_targets = package_meta.targets.iter().filter(|target| {
        target.kind.iter().any(|kind| kind == &TargetKind::CDyLib)
    });

    let cdylib_target = cdylib_targets.next().ok_or_else(|| {
        anyhow!(
            "Could not find a cdylib target in manifest of package {:?}",
            package_meta.name
        )
    })?;

    if cdylib_targets.next().is_some() {
        return Err(anyhow!(
            "Found multiple cdylib targets in manifest of package {:?}",
            package_meta.name
        ));
    }

    let source = format!(
        "{prefix}{lib_name}{suffix}",
        prefix = env::consts::DLL_PREFIX,
        lib_name = &cdylib_target.name,
        suffix = env::consts::DLL_SUFFIX
    )
    .parse::<NodeNameBuf>()
    .unwrap();

    let dest = format!(
        "{lib_name}{suffix}",
        lib_name = &cdylib_target.name,
        suffix = if cfg!(target_os = "windows") { ".dll" } else { ".so" }
    )
    .parse::<NodeNameBuf>()
    .unwrap();

    force_rename(&artifact_dir.join(&source), &artifact_dir.join(&dest))
        .context("Failed to rename the library")
}

fn copy_dir(src_dir: &AbsPath, dst_dir: &AbsPath) -> anyhow::Result<()> {
    assert!(!fs::exists(dst_dir)?);

    fs::create_dir_all(dst_dir)?;

    for entry in fs::read_dir(src_dir)? {
        let entry = entry?;

        let os_entry_name = entry.file_name();

        let entry_name = os_entry_name
            .to_str()
            .map(<&NodeName>::try_from)
            .context("Invalid file name")??;

        let src = src_dir.join(entry_name);
        let dst = dst_dir.join(entry_name);
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_dir(&src, &dst)?;
        } else if file_type.is_file() {
            fs::copy(&src, &dst)?;
        } else if file_type.is_symlink() {
            copy_symlink(&src, &dst)?;
        }
    }

    Ok(())
}

fn copy_symlink(src: &AbsPath, dst: &AbsPath) -> anyhow::Result<()> {
    let link_target = fs::read_link(src)?;

    let link_src = if link_target.is_absolute() {
        AbsPathBuf::try_from(link_target)?
    } else {
        let src = path::Path::new(src.as_str());
        let target_src = fs::canonicalize(
            src.parent().expect("not root").join(link_target),
        )?;
        AbsPathBuf::try_from(target_src)?
    };

    let file_type = fs::metadata(&link_src)?.file_type();

    if file_type.is_dir() {
        copy_dir(&link_src, dst)?;
    } else if file_type.is_file() {
        fs::copy(&link_src, dst)?;
    } else if file_type.is_symlink() {
        copy_symlink(&link_src, dst)?;
    }

    Ok(())
}

fn force_rename(src: &AbsPath, dst: &AbsPath) -> anyhow::Result<()> {
    if fs::metadata(dst).is_ok() {
        fs::remove_file(dst)?;
    }
    fs::rename(src, dst)?;
    Ok(())
}
