use core::error::Error;

use crate::fs::{AbsPath, Fs, FsNode, NodeName};

/// TODO: docs.
pub trait Symlink {
    /// TODO: docs.
    type Fs: Fs;

    /// TODO: docs.
    type DeleteError: Error;

    /// TODO: docs.
    type FollowError: Error;

    /// TODO: docs.
    type MetadataError: Error;

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
    fn id(&self) -> <Self::Fs as Fs>::NodeId;

    /// TODO: docs.
    fn meta(
        &self,
    ) -> impl Future<Output = Result<<Self::Fs as Fs>::Metadata, Self::MetadataError>>;

    /// TODO: docs.
    fn name(&self) -> &NodeName {
        self.path().node_name().expect("path is not root")
    }

    /// TODO: docs.
    fn path(&self) -> &AbsPath;
}
