use core::{fmt, iter, str};
use std::borrow::Cow;
use std::env;

use abs_path::{AbsPath, AbsPathBuf, NodeNameBuf, node};
use anyhow::{Context, anyhow};
use cargo_metadata::TargetKind;
use xshell::cmd;

use crate::neovim::CARGO_TOML_META;

#[derive(Debug, Copy, Clone, clap::Args)]
pub(crate) struct BuildArgs {
    /// Build the plugin in release mode.
    #[clap(long, short)]
    release: bool,

    /// Build the plugin for the latest nightly version of Neovim.
    #[clap(long)]
    nightly: bool,
}

pub(crate) fn build(args: BuildArgs) -> anyhow::Result<()> {
    let sh = xshell::Shell::new()?;
    build_plugin(args, &sh)?;
    fix_library_name()?;
    Ok(())
}

fn build_plugin(args: BuildArgs, sh: &xshell::Shell) -> anyhow::Result<()> {
    struct Arg<'a>(Cow<'a, str>);

    impl AsRef<std::ffi::OsStr> for Arg<'_> {
        fn as_ref(&self) -> &std::ffi::OsStr {
            self.0.as_ref().as_ref()
        }
    }

    let package_meta = &CARGO_TOML_META;

    // Setting the artifact directory is still unstable.
    let artifact_dir_args = ["-Zunstable-options", "--artifact-dir"]
        .into_iter()
        .map(Cow::Borrowed)
        .chain(iter::once(Cow::Owned(artifact_dir().to_string())));

    // Specify which package to build.
    let package_args =
        ["--package", &package_meta.name].into_iter().map(Cow::Borrowed);

    let is_nightly = args.nightly
        || NeovimVersion::detect(sh).map(|v| v.is_nightly()).unwrap_or(false);

    let feature_args = is_nightly
        .then_some("--features=neovim-nightly")
        .into_iter()
        .map(Cow::Borrowed);

    let profile_args =
        args.release.then_some("--release").into_iter().map(Cow::Borrowed);

    let args = artifact_dir_args
        .chain(package_args)
        .chain(feature_args)
        .chain(profile_args)
        .map(Arg);

    cmd!(sh, "cargo build {args...}").run()?;

    Ok(())
}

#[allow(clippy::unwrap_used)]
fn fix_library_name() -> anyhow::Result<()> {
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

    force_rename(artifact_dir().push(source), artifact_dir().push(dest))
        .context("Failed to rename the library")
}

fn artifact_dir() -> AbsPathBuf {
    crate::WORKSPACE_ROOT.join(node!("lua"))
}

fn force_rename(src: &AbsPath, dst: &AbsPath) -> anyhow::Result<()> {
    if std::fs::metadata(dst).is_ok() {
        std::fs::remove_file(dst)?;
    }
    std::fs::rename(src, dst)?;
    Ok(())
}

/// The possible Neovim versions our plugin can be built for.
#[derive(Debug, Copy, Clone)]
enum NeovimVersion {
    /// The latest stable version.
    ZeroDotEleven,

    /// The latest nightly version.
    Nightly,
}

struct SemanticVersion {
    major: u8,
    minor: u8,
    patch: u8,
}

impl NeovimVersion {
    fn detect(sh: &xshell::Shell) -> Option<Self> {
        let version = "--version";
        let stdout = cmd!(sh, "nvim {version}").read().ok()?;
        let (_, rest) = stdout.lines().next()?.split_once("NVIM v")?;
        rest.parse::<Self>().ok()
    }

    fn is_nightly(self) -> bool {
        matches!(self, Self::Nightly)
    }
}

impl str::FromStr for NeovimVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let nightly_suffix = "-dev";
        let is_nightly = s.ends_with(nightly_suffix);
        let version = s
            [..s.len() - (is_nightly as usize) * nightly_suffix.len()]
            .parse::<SemanticVersion>()
            .context("Failed to parse Neovim version")?;
        if version.major == 0 && version.minor == 11 {
            Ok(Self::ZeroDotEleven)
        } else if version.major == 0 && version.minor == 12 && is_nightly {
            Ok(Self::Nightly)
        } else {
            Err(anyhow!(
                "Unsupported Neovim version: {version}{}",
                if is_nightly { nightly_suffix } else { "" }
            ))
        }
    }
}

impl fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl str::FromStr for SemanticVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('.');
        let major =
            parts.next().ok_or_else(|| anyhow!("major version is missing"))?;
        let minor =
            parts.next().ok_or_else(|| anyhow!("minor version is missing"))?;
        let patch =
            parts.next().ok_or_else(|| anyhow!("patch version is missing"))?;
        if parts.next().is_some() {
            return Err(anyhow!("too many version parts"));
        }
        Ok(Self {
            major: major.parse()?,
            minor: minor.parse()?,
            patch: patch.parse()?,
        })
    }
}
