use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::{env, fmt, fs};

use crate::{TestError, TestResult};

/// TODO: docs
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
    Command::new("cargo")
        .arg("build")
        .args(BuildProfile::from_env().is_release().then_some("--release"))
        // We have to use a different target directory or we'll get a deadlock.
        // See https://github.com/rust-lang/cargo/issues/6412.
        .args(["--target-dir", target_dir().display().to_string().as_ref()])
        .arg("--features")
        .arg(NvimVersion::detect()?.as_feature().to_string())
        .status()
        .map(|_| ())
        .map_err(Into::into)
}

/// TODO: docs
pub(crate) fn target_dir() -> PathBuf {
    crate_target_dir()
        .join("nvimx-tests")
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
    #[cfg(feature = "test_macro")]
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }

    pub(crate) fn from_env() -> Self {
        let profile = env::var("PROFILE").expect("$PROFILE env var not set");

        match profile.as_str() {
            "debug" => Self::Debug,
            "release" => Self::Release,
            _ => unreachable!("unknown profile {profile:?}"),
        }
    }

    fn is_release(&self) -> bool {
        matches!(self, Self::Release)
    }
}

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
    /// TODO: docs
    fn as_feature(&self) -> impl fmt::Display {
        NvimVersionFeature(*self)
    }

    /// TODO: docs
    fn detect() -> Result<Self, TestError> {
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
