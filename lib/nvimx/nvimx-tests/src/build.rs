use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::{env, fmt, fs};

use crate::{TestError, TestResult};

/// The name of the file used to store the current [`BuildProfile`].
const PROFILE_FILE_NAME: &str = "build_profile";

/// Builds the crate before running the tests.
///
/// One problem with testing Neovim plugins written in Rust is that the user
/// has to remember to run `cargo build` after every change to make sure the
/// generated dynamic library that Neovim loads actually contains the latest
/// version of the test's body.
///
/// A way to solve this is to put all the tests in a separate `cdylib` crate
/// with a `build.rs` script that builds the crate. Cargo will then take care
/// of automatically calling that script anytime the crate or any of its
/// dependencies change.
///
/// This is exactly what this function does. It's meant to be used as the body
/// of a build script, like so:
///
/// ```ignore
/// // build.rs
/// fn main() {
///    nvimx::tests::build();
/// }
/// ```
///
/// With that in place, you can run your tests with `cargo test` as you
/// normally would, without having to run `cargo build` first.
///
/// Note that the tests **must** be annotated with `#[nvimx::test]` when using
/// this setup. Using `#[nvim_oxi::test]` will not work.
///
/// # Panics
///
/// Panics if called outside of a `build.rs` script.
pub fn build() {
    match BuildingGuard::new().guard(build_crate) {
        Ok(Ok(())) | Err(BuildGuardError::AlreadyBuilding) => Ok(()),

        Ok(Err(err))
        | Err(BuildGuardError::CouldntLock(err))
        | Err(BuildGuardError::CouldntUnlock(err)) => Err(err),
    }
    .unwrap_or_else(|err| panic!("couldn't build tests: {err}"))
}

fn build_crate() -> TestResult {
    let profile = BuildProfile::from_env();

    Command::new("cargo")
        .arg("build")
        .args(profile.is_release().then_some("--release"))
        // We have to use a different target directory or we'll get a deadlock.
        // See https://github.com/rust-lang/cargo/issues/6412.
        .args(["--target-dir", target_dir().display().to_string().as_ref()])
        .arg("--features")
        .arg(NvimVersion::from_env()?.as_feature().to_string())
        .status()
        .map(|_| ())?;

    fs::write(target_dir().join(PROFILE_FILE_NAME), profile.as_str())?;

    Ok(())
}

/// Returns the subdirectory of the crate's `/target` directory used as the
/// test crate's target directory.
pub(crate) fn target_dir() -> PathBuf {
    crate_target_dir()
        .join("nvimx-tests")
        // We namespace this by package name because there may be multiple test
        // crates in the same workspace.
        .join(env::var("CARGO_PKG_NAME").expect("set when building"))
}

/// Returns the `/target` directory where cargo will place the build artifacts
/// for the current crate.
fn crate_target_dir() -> PathBuf {
    let manifest = env::var("CARGO_MANIFEST_DIR").expect("set when building");
    nvim_oxi::tests::target_dir(Path::new(&manifest))
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum BuildProfile {
    Debug,
    Release,
}

impl BuildProfile {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }

    /// Returns the profile used the last time the crate was built.
    pub(crate) fn current() -> Self {
        let profile_path = target_dir().join(PROFILE_FILE_NAME);

        fs::read_to_string(profile_path)
            .map(|profile| Self::from_str(profile.trim()))
            .unwrap_or_else(|err| panic!("{err}"))
    }

    pub(crate) fn from_env() -> Self {
        env::var("PROFILE")
            .map(|profile| Self::from_str(&profile))
            .expect("$PROFILE env var not set")
    }

    fn from_str(s: &str) -> Self {
        match s {
            "debug" => Self::Debug,
            "release" => Self::Release,
            _ => panic!("unknown profile {s:?}"),
        }
    }

    fn is_release(&self) -> bool {
        matches!(self, Self::Release)
    }
}

/// A guard that ensures we don't try to build the crate recursively.
///
/// The [`build`] function is called inside a `build.rs`, and internally it
/// calls `cargo build`, which would execute the `build.rs` script, and so on.
///
/// To avoid that we create a lock file in the target directory on the first
/// execution, skip the build if it exists, and remove it once the build
/// completes.
struct BuildingGuard {
    lock_file_path: PathBuf,
}

impl BuildingGuard {
    fn guard<R>(&self, fun: impl FnOnce() -> R) -> Result<R, BuildGuardError> {
        if self.is_locked() {
            Err(BuildGuardError::AlreadyBuilding)
        } else {
            self.lock().map_err(BuildGuardError::CouldntLock)?;
            let res = fun();
            self.unlock().map_err(BuildGuardError::CouldntUnlock)?;
            Ok(res)
        }
    }

    fn is_locked(&self) -> bool {
        self.lock_file_path.exists()
    }

    fn lock(&self) -> Result<(), TestError> {
        let parent = self
            .lock_file_path
            .parent()
            .expect("lock file is inside a directory");

        fs::create_dir_all(parent)?;
        fs::write(&self.lock_file_path, "").map_err(Into::into)
    }

    fn new() -> Self {
        Self { lock_file_path: target_dir().join(".nomad_build_lock") }
    }

    fn unlock(&self) -> Result<(), TestError> {
        fs::remove_file(&self.lock_file_path)?;
        Ok(())
    }
}

#[derive(Debug)]
enum BuildGuardError {
    CouldntLock(TestError),
    CouldntUnlock(TestError),
    AlreadyBuilding,
}

impl fmt::Display for BuildGuardError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("already building")
    }
}

impl std::error::Error for BuildGuardError {}

#[derive(Copy, Clone, Debug)]
struct NvimVersion {
    is_nightly: bool,
    major: u8,
    minor: u8,
    patch: u8,
}

impl NvimVersion {
    /// Returns the name of the feature corresponding to this version.
    ///
    /// Note that this forces the user to:
    ///
    /// a) explicitly add a feature in their `Cargo.toml` corresponding to the
    ///    their Neovim version, even if they don't want to run their tests
    ///    against multiple versions;
    ///
    /// b) use the `neovim-*` scheme when naming the feature.
    fn as_feature(&self) -> impl fmt::Display {
        NvimVersionFeature(*self)
    }

    fn from_env() -> Result<Self, TestError> {
        let stdout = Command::new("nvim").arg("--version").output()?.stdout;
        let stdout = String::from_utf8_lossy(&stdout);
        Self::from_stdout(&stdout)
    }

    /// Creates a [`NvimVersion`] by parsing the output of the `nvim --version`
    /// command.
    fn from_stdout(stdout: &str) -> Result<Self, TestError> {
        let mut this =
            Self { is_nightly: false, major: 0, minor: 0, patch: 0 };

        let Some(first_line) = stdout.lines().next() else {
            return Err("stdout is empty".into());
        };

        let Some((_, rest)) = first_line.split_once("NVIM v") else {
            return Err("failed to parse version".into());
        };

        let Some((major, rest)) = rest.split_once('.') else {
            return Err("failed to parse major version".into());
        };

        this.major = u8::from_str(major)?;

        let Some((minor, rest)) = rest.split_once('.') else {
            return Err("failed to parse minor version".into());
        };

        this.minor = u8::from_str(minor)?;

        match rest.split_once('-') {
            Some((patch, rest)) => {
                this.patch = u8::from_str(patch)?;
                this.is_nightly = rest.starts_with("dev");
            },

            None => {
                this.patch = u8::from_str(rest)?;
            },
        }

        Ok(this)
    }
}

struct NvimVersionFeature(NvimVersion);

impl fmt::Display for NvimVersionFeature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self(version) = self;

        f.write_str("neovim-")?;

        if version.is_nightly {
            f.write_str("nightly")?;
        } else {
            write!(f, "{}-{}", version.major, version.minor)?;
        }

        Ok(())
    }
}
