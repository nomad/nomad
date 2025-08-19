use std::io;
use std::path::PathBuf;

use abs_path::AbsPath;
use fs::Fs;
use fs::os::OsFs;

use crate::TempDir;

/// TODO: docs.
pub trait FsExt {
    /// TODO: docs.
    fn tempdir(
        &self,
    ) -> impl Future<Output = Result<TempDir, TempDirError>> + Send;
}

/// TODO: docs.
#[derive(Debug, derive_more::Display, cauchy::Error)]
#[display("{_0}")]
pub enum TempDirError {
    /// TODO: docs.
    CreateDir(io::Error),

    /// TODO: docs.
    GetDir(<OsFs as Fs>::NodeAtPathError),

    /// The path the temporary directory was created at is not valid UTF-8.
    #[display("{_0:?} is not valid UTF-8")]
    NonUtf8Path(PathBuf),
}

impl FsExt for OsFs {
    async fn tempdir(&self) -> Result<TempDir, TempDirError> {
        let temp_dir = tempdir_inner::TempDir::new("")
            .map_err(TempDirError::CreateDir)?;

        let temp_dir_path = match <&AbsPath>::try_from(temp_dir.path()) {
            Ok(path) => path,
            Err(abs_path::AbsPathFromPathError::NotAbsolute) => {
                unreachable!("the path is absolute")
            },
            Err(abs_path::AbsPathFromPathError::NotUtf8) => {
                return Err(TempDirError::NonUtf8Path(
                    temp_dir.path().to_owned(),
                ));
            },
        };

        let os_dir = self
            .node_at_path(temp_dir_path)
            .await
            .map_err(TempDirError::GetDir)?
            .expect("just created the directory")
            .unwrap_directory();

        Ok(TempDir::new(temp_dir, os_dir))
    }
}
