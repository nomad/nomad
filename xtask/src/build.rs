use core::{fmt, iter, str};
use std::borrow::Cow;
use std::env;

use anyhow::{anyhow, Context};
use futures_executor::block_on;
use nvimx::fs::os_fs::OsFs;
use nvimx::fs::{AbsPath, AbsPathBuf, FsNodeName, FsNodeNameBuf};
use root_finder::markers;
use xshell::cmd;

/// The desired name of the library to placed in the `/lua` directory.
const LIBRARY_NAME: &str = "nomad";

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
    Build::new(xshell::Shell::new()?)
        .find_project_root()?
        .parse_package()?
        .build_plugin(args)?
        .fix_library_name()
}

struct Build<State> {
    sh: xshell::Shell,
    state: State,
}

struct FindProjectRoot;

struct ParsePackage {
    project_root: AbsPathBuf,
}

struct BuildPlugin {
    project_root: AbsPathBuf,
    package: cargo_metadata::Package,
}

struct FixLibraryName {
    project_root: AbsPathBuf,
    package: cargo_metadata::Package,
}

impl Build<FindProjectRoot> {
    fn find_project_root(self) -> anyhow::Result<Build<ParsePackage>> {
        let project_root = self.state.call(&self.sh)?;
        Ok(Build { sh: self.sh, state: ParsePackage { project_root } })
    }

    fn new(sh: xshell::Shell) -> Self {
        Self { sh, state: FindProjectRoot }
    }
}

impl Build<ParsePackage> {
    fn parse_package(self) -> anyhow::Result<Build<BuildPlugin>> {
        let package = self.state.call()?;
        Ok(Build {
            sh: self.sh,
            state: BuildPlugin {
                project_root: self.state.project_root,
                package,
            },
        })
    }
}

impl Build<BuildPlugin> {
    fn build_plugin(
        self,
        args: BuildArgs,
    ) -> anyhow::Result<Build<FixLibraryName>> {
        self.state.call(args, &self.sh)?;
        Ok(Build {
            sh: self.sh,
            state: FixLibraryName {
                project_root: self.state.project_root,
                package: self.state.package,
            },
        })
    }
}

impl Build<FixLibraryName> {
    fn fix_library_name(self) -> anyhow::Result<()> {
        self.state.call()
    }
}

impl FindProjectRoot {
    fn call(&self, sh: &xshell::Shell) -> anyhow::Result<AbsPathBuf> {
        let current_dir = sh.current_dir();
        let current_dir = <&AbsPath>::try_from(&*current_dir)?;
        let root_finder = root_finder::Finder::new(OsFs);
        block_on(root_finder.find_root(current_dir, markers::Git))?
            .ok_or_else(|| anyhow!("Could not find the project root"))
    }
}

impl ParsePackage {
    fn call(&self) -> anyhow::Result<cargo_metadata::Package> {
        let cargo_dot_toml = {
            let mut root = self.project_root.to_owned();
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
}

impl BuildPlugin {
    fn call(&self, args: BuildArgs, sh: &xshell::Shell) -> anyhow::Result<()> {
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
            .chain(iter::once(Cow::Owned(
                artifact_dir(&self.project_root).to_string(),
            )));

        // Specify which package to build.
        let package_args =
            ["--package", &self.package.name].into_iter().map(Cow::Borrowed);

        let is_nightly = args.nightly
            || NeovimVersion::detect(sh)
                .map(|v| v.is_nightly())
                .unwrap_or(false);

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
}

impl FixLibraryName {
    #[allow(clippy::unwrap_used)]
    fn call(&self) -> anyhow::Result<()> {
        let mut cdylib_targets =
            self.package.targets.iter().filter(|target| {
                target.kind.iter().any(|kind| kind == "cdylib")
            });
        let cdylib_target = cdylib_targets.next().ok_or_else(|| {
            anyhow!(
                "Could not find a cdylib target in manifest of package {:?}",
                self.package.name
            )
        })?;
        if cdylib_targets.next().is_some() {
            return Err(anyhow!(
                "Found multiple cdylib targets in manifest of package {:?}",
                self.package.name
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
        force_rename(
            artifact_dir(&self.project_root).push(source),
            artifact_dir(&self.project_root).push(dest),
        )
        .context("Failed to rename the library")
    }
}

fn artifact_dir(project_root: &AbsPath) -> AbsPathBuf {
    let mut dir = project_root.to_owned();
    #[allow(clippy::unwrap_used)]
    dir.push(<&FsNodeName>::try_from("lua").unwrap());
    dir
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
