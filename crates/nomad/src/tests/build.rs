use std::fmt::{self, Display};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::{env, fs};

use super::{TestError, TestResult};

/// TODO: docs
pub fn build_script() -> TestResult {
    match BuildingGuard::new().guard(build_crate) {
        Ok(Ok(())) | Err(BuildGuardError::AlreadyBuilding) => Ok(()),

        Ok(Err(err))
        | Err(BuildGuardError::CouldntLock(err))
        | Err(BuildGuardError::CouldntUnlock(err)) => Err(err),
    }
}

fn build_crate() -> TestResult {
    Command::new("cargo")
        .arg("build")
        // We have to use a different target directory or we'll get a deadlock.
        // See https://github.com/rust-lang/cargo/issues/6412.
        .args([
            "--target-dir",
            build_target_dir().display().to_string().as_ref(),
        ])
        .arg("--features")
        .arg(NvimVersion::detect()?.as_feature().to_string())
        .status()
        .map(|_| ())
        .map_err(Into::into)
}

fn build_target_dir() -> PathBuf {
    workspace_target_dir()
        .join("nvim-tests")
        .join(env::var("CARGO_PKG_NAME").expect("set when building"))
}

fn workspace_target_dir() -> PathBuf {
    let manifest = env::var("CARGO_MANIFEST_DIR").expect("set when building");
    nvim::tests::target_dir(Path::new(&manifest))
}

/// TODO: docs
pub fn library_path(crate_name: &str) -> PathBuf {
    let library_name = format!(
        "{prefix}{crate_name}{suffix}",
        prefix = env::consts::DLL_PREFIX,
        suffix = env::consts::DLL_SUFFIX,
    );

    build_target_dir().join("debug").join(library_name)
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
        Self { lock_file_path: build_target_dir().join(".nomad_build_lock") }
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

impl Display for BuildGuardError {
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
    pub fn as_feature(&self) -> impl Display {
        NvimVersionFeature(*self)
    }

    /// TODO: docs
    pub fn detect() -> Result<Self, TestError> {
        let stdout = Command::new("nvim").arg("--version").output()?.stdout;
        let stdout = String::from_utf8_lossy(&stdout);
        Self::from_stdout(&stdout)
    }

    /// Creates a [`NvimVersion`] by parsing the output of the `nvim --version`
    /// command.
    pub fn from_stdout(stdout: &str) -> Result<Self, TestError> {
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

impl Display for NvimVersionFeature {
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
