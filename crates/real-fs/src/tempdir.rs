use core::ops::Deref;

use crate::Directory;

/// TODO: docs.
pub struct TempDirectory {
    pub(crate) inner: Directory,

    /// We need to keep the inner handle around so that the directory is
    /// deleted when `Self` is dropped.
    pub(crate) _handle: tempdir::TempDir,
}

impl Deref for TempDirectory {
    type Target = Directory;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
