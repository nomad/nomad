use core::error::Error;

use crate::fs::{self, AbsPath, FsNode, FsNodeName};

/// TODO: docs.
pub trait Symlink {
    /// TODO: docs.
    type Fs: fs::Fs;

    /// TODO: docs.
    type DeleteError: Error;

    /// TODO: docs.
    type FollowError: Error;

    /// TODO: docs.
    fn delete(self) -> impl Future<Output = Result<(), Self::DeleteError>>;

    /// TODO: docs.
    fn follow(
        &self,
    ) -> impl Future<Output = Result<Option<FsNode<Self::Fs>>, Self::FollowError>>;

    /// TODO: docs.
    fn follow_recursively(
        &self,
    ) -> impl Future<Output = Result<Option<FsNode<Self::Fs>>, Self::FollowError>>;

    /// TODO: docs.
    fn name(&self) -> &FsNodeName {
        self.path().node_name().expect("path is not root")
    }

    /// TODO: docs.
    fn path(&self) -> &AbsPath;
}
