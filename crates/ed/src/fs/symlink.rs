use core::error::Error;

use futures_util::Stream;

use crate::fs::{self, AbsPath, Fs, FsNode, NodeDeletion, NodeMove, NodeName};

/// TODO: docs.
pub trait Symlink: Send + Sync {
    /// TODO: docs.
    type EventStream: Stream<Item = SymlinkEvent<Self::Fs>> + Send + Unpin;

    /// TODO: docs.
    type Fs: Fs;

    /// TODO: docs.
    type DeleteError: Error + Send;

    /// TODO: docs.
    type FollowError: Error + Send;

    /// TODO: docs.
    type ParentError: Error + Send;

    /// TODO: docs.
    type ReadError: Error + Send;

    /// TODO: docs.
    fn delete(
        self,
    ) -> impl Future<Output = Result<(), Self::DeleteError>> + Send;

    /// TODO: docs.
    fn follow(
        &self,
    ) -> impl Future<Output = Result<Option<FsNode<Self::Fs>>, Self::FollowError>>;

    /// TODO: docs.
    fn follow_recursively(
        &self,
    ) -> impl Future<Output = Result<Option<FsNode<Self::Fs>>, Self::FollowError>>;

    /// TODO: docs.
    #[inline]
    fn id(&self) -> <Self::Fs as Fs>::NodeId {
        fs::Metadata::id(&self.meta())
    }

    /// TODO: docs.
    fn meta(&self) -> <Self::Fs as Fs>::Metadata;

    /// TODO: docs.
    fn name(&self) -> &NodeName {
        self.path().node_name().expect("path is not root")
    }

    /// TODO: docs.
    fn parent(
        &self,
    ) -> impl Future<
        Output = Result<<Self::Fs as Fs>::Directory, Self::ParentError>,
    > + Send;

    /// TODO: docs.
    fn path(&self) -> &AbsPath;

    /// TODO: docs.
    fn read_path(
        &self,
    ) -> impl Future<Output = Result<String, Self::ReadError>> + Send;

    /// TODO: docs.
    fn watch(&self) -> Self::EventStream;
}

/// TODO: docs.
pub enum SymlinkEvent<Fs: fs::Fs> {
    /// TODO: docs.
    Deletion(NodeDeletion<Fs>),

    /// TODO: docs.
    Move(NodeMove<Fs>),
}
