use std::io;

use e31e::fs::AbsPath;

use crate::Marker;

/// TODO: docs.
pub struct Git;

impl Git {
    const GIT_DIR: &'static str = ".git";
}

impl Marker for Git {
    async fn matches<F>(
        &self,
        path: &AbsPath,
        // metadata: &F::Metadata,
        fs: &F,
    ) -> io::Result<bool> {
        // let is_dir = fs.is_dir(&metadata).await?;
        // let file_name = path.file_name().expect("matches called on root dir");
        // Ok(is_di && file_name == Self::GIT_DIR)
        todo!();
    }
}
