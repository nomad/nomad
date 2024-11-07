use core::{fmt, iter, str};
use std::borrow::Cow;
use std::env;

use anyhow::{anyhow, Context};
use fs::os_fs::OsFs;
use fs::{AbsPath, AbsPathBuf, FsNodeName, FsNodeNameBuf};
use futures_executor::block_on;
use root_finder::markers;
use xshell::cmd;

/// The desired name of the library to placed in the `/lua` directory.
const LIBRARY_NAME: &str = "nomad";

pub(crate) fn build(release: bool) -> anyhow::Result<()> {
    let sh = xshell::Shell::new()?;
    let project_root = find_project_root(&sh)?;
    let package = parse_package(&project_root)?;
    let nvim_version = detect_nvim_version(&sh)?;
    build_plugin(&project_root, &package.name, nvim_version, release, &sh)?;
    fix_library_name(&project_root, &package)?;
    Ok(())
}

fn find_project_root(sh: &xshell::Shell) -> anyhow::Result<AbsPathBuf> {
    let current_dir = sh.current_dir();
    let current_dir = <&AbsPath>::try_from(&*current_dir)?;
    let root_finder = root_finder::Finder::new(OsFs);
    block_on(root_finder.find_root(current_dir, markers::Git))?
        .ok_or_else(|| anyhow!("Could not find the project root"))
}

fn parse_package(
    project_root: &AbsPath,
) -> anyhow::Result<cargo_metadata::Package> {
    let cargo_dot_toml = {
        let mut root = project_root.to_owned();
        #[allow(clippy::unwrap_used)]
        root.push(<&FsNodeName>::try_from("Cargo.toml").unwrap());
        root
    };
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(cargo_dot_toml.clone())
        .exec()?;
    metadata.root_package().cloned().ok_or_else(|| {
        anyhow!(
            "Could not find the root package for manifest at \
             {cargo_dot_toml:?}"
        )
    })
}

fn detect_nvim_version(sh: &xshell::Shell) -> anyhow::Result<NeovimVersion> {
    let version = "--version";
    let stdout = cmd!(sh, "nvim {version}").read()?;
    stdout
        .lines()
        .next()
        .ok_or_else(|| anyhow!("Couldn't get Neovim version"))?
        .split_once("NVIM v")
        .map(|(_, rest)| rest.parse::<NeovimVersion>())
        .transpose()?
        .ok_or_else(|| anyhow!("Failed to parse Neovim version"))
}

#[allow(clippy::too_many_arguments)]
fn build_plugin(
    project_root: &AbsPath,
    package_name: &str,
    nvim_version: NeovimVersion,
    release: bool,
    sh: &xshell::Shell,
) -> anyhow::Result<()> {
    struct Arg<'a>(Cow<'a, str>);

    impl AsRef<std::ffi::OsStr> for Arg<'_> {
        fn as_ref(&self) -> &std::ffi::OsStr {
            self.0.as_ref().as_ref()
        }
    }

    // Setting the artifact directory is still unstable.
    let artifact_dir_args = ["-Zunstable-options", "--artifact-dir"]
        .into_iter()
        .map(Cow::Borrowed)
        .chain(iter::once(Cow::Owned(artifact_dir(project_root).to_string())));

    // Specify which package to build.
    let package_args =
        ["--package", package_name].into_iter().map(Cow::Borrowed);

    // Compile the plugin for Nightly if the user is using a Nightly version of
    // Neovim.
    let feature_args = nvim_version
        .is_nightly()
        .then_some(["--features", "neovim-nightly"])
        .into_iter()
        .flatten()
        .map(Cow::Borrowed);

    let profile_args =
        release.then_some("--release").into_iter().map(Cow::Borrowed);

    let args = artifact_dir_args
        .chain(package_args)
        .chain(feature_args)
        .chain(profile_args)
        .map(Arg);

    cmd!(sh, "cargo build {args...}").run()?;

    Ok(())
}

#[allow(clippy::unwrap_used)]
fn fix_library_name(
    project_root: &AbsPathBuf,
    package: &cargo_metadata::Package,
) -> anyhow::Result<()> {
    let mut cdylib_targets = package
        .targets
        .iter()
        .filter(|target| target.kind.iter().any(|kind| kind == "cdylib"));
    let cdylib_target = cdylib_targets.next().ok_or_else(|| {
        anyhow!(
            "Could not find a cdylib target in manifest of package {:?}",
            package.name
        )
    })?;
    if cdylib_targets.next().is_some() {
        return Err(anyhow!(
            "Found multiple cdylib targets in manifest of package {:?}",
            package.name
        ));
    }
    let source = format!(
        "{prefix}{source_name}{suffix}",
        prefix = env::consts::DLL_PREFIX,
        source_name = &cdylib_target.name,
        suffix = env::consts::DLL_SUFFIX
    )
    .parse::<FsNodeNameBuf>()
    .unwrap();
    let dest = format!(
        "{dest_name}{suffix}",
        dest_name = LIBRARY_NAME,
        suffix = if cfg!(target_os = "windows") { ".dll" } else { ".so" }
    )
    .parse::<FsNodeNameBuf>()
    .unwrap();
    std::fs::rename(
        artifact_dir(project_root).push(source),
        artifact_dir(project_root).push(dest),
    )
    .context("Failed to rename the library")
}

fn artifact_dir(project_root: &AbsPath) -> AbsPathBuf {
    let mut dir = project_root.to_owned();
    #[allow(clippy::unwrap_used)]
    dir.push(<&FsNodeName>::try_from("lua").unwrap());
    dir
}

/// The possible Neovim versions our plugin can be built for.
#[derive(Debug)]
enum NeovimVersion {
    /// The latest stable version.
    ZeroDotTen,

    /// The latest nightly version.
    Nightly,
}

struct SemanticVersion {
    major: u8,
    minor: u8,
    patch: u8,
}

impl NeovimVersion {
    fn is_nightly(&self) -> bool {
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
        if version.major == 0 && version.minor == 10 {
            Ok(Self::ZeroDotTen)
        } else if version.major == 0 && version.minor == 11 && is_nightly {
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
