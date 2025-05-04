use core::error::Error;

use abs_path::AbsPathBuf;
use futures_lite::Stream;

use crate::fs::{self, AbsPath, Fs, NodeName};

/// TODO: docs.
pub trait Directory: Send + Sync + Sized {
    /// TODO: docs.
    type EventStream: Stream<Item = DirectoryEvent<Self::Fs>> + Send + Unpin;

    /// TODO: docs.
    type Fs: Fs;

    /// TODO: docs.
    type CreateDirectoryError: Error + Send;

    /// TODO: docs.
    type CreateFileError: Error + Send;

    /// TODO: docs.
    type CreateSymlinkError: Error + Send;

    /// TODO: docs.
    type ClearError: Error + Send;

    /// TODO: docs.
    type DeleteError: Error + Send;

    /// TODO: docs.
    type ParentError: Error + Send;

    /// TODO: docs.
    type ReadEntryError: Error + Send;

    /// TODO: docs.
    type ReadError: Error + Send;

    /// TODO: docs.
    fn create_directory(
        &self,
        directory_name: &NodeName,
    ) -> impl Future<
        Output = Result<
            <Self::Fs as Fs>::Directory,
            Self::CreateDirectoryError,
        >,
    > + Send;

    /// TODO: docs.
    fn create_file(
        &self,
        file_name: &NodeName,
    ) -> impl Future<
        Output = Result<<Self::Fs as Fs>::File, Self::CreateFileError>,
    > + Send;

    /// TODO: docs.
    fn create_symlink(
        &self,
        symlink_name: &NodeName,
        target_path: &str,
    ) -> impl Future<
        Output = Result<<Self::Fs as Fs>::Symlink, Self::CreateSymlinkError>,
    > + Send;

    /// TODO: docs.
    fn clear(&self) -> impl Future<Output = Result<(), Self::ClearError>>;

    /// TODO: docs.
    fn delete(
        self,
    ) -> impl Future<Output = Result<(), Self::DeleteError>> + Send;

    /// TODO: docs.
    #[inline]
    fn id(&self) -> <Self::Fs as Fs>::NodeId {
        fs::Metadata::id(&self.meta())
    }

    /// TODO: docs.
    fn meta(&self) -> <Self::Fs as Fs>::Metadata;

    /// TODO: docs.
    #[inline]
    fn name(&self) -> Option<&NodeName> {
        self.path().node_name()
    }

    /// TODO: docs.
    fn parent(
        &self,
    ) -> impl Future<
        Output = Result<
            Option<<Self::Fs as Fs>::Directory>,
            Self::ParentError,
        >,
    > + Send;

    /// TODO: docs.
    fn path(&self) -> &AbsPath;

    /// TODO: docs.
    #[allow(clippy::type_complexity)]
    fn read(
        &self,
    ) -> impl Future<
        Output = Result<
            impl Stream<
                Item = Result<
                    <Self::Fs as Fs>::Metadata,
                    Self::ReadEntryError,
                >,
            > + Send
            + use<Self>,
            Self::ReadError,
        >,
    > + Send;

    /// TODO: docs.
    #[inline]
    fn replicate_from<Src: Directory>(
        &self,
        _src: &Src,
    ) -> impl Future<Output = Result<(), ReplicateError<Self::Fs, Src::Fs>>> + Send
    {
        async move {
            todo!();
        }
    }

    /// TODO: docs.
    fn watch(&self) -> Self::EventStream;
}

/// TODO: docs.
pub enum DirectoryEvent<Fs: fs::Fs> {
    /// TODO: docs.
    Creation(NodeCreation<Fs>),

    /// TODO: docs.
    Deletion(NodeDeletion<Fs>),

    /// TODO: docs.
    Move(NodeMove<Fs>),
}

/// TODO: docs.
pub struct NodeCreation<Fs: fs::Fs> {
    /// TODO: docs.
    pub node_id: Fs::NodeId,

    /// TODO: docs.
    pub node_path: AbsPathBuf,

    /// TODO: docs.
    pub parent_id: Fs::NodeId,
}

/// TODO: docs.
pub struct NodeDeletion<Fs: fs::Fs> {
    /// The ID of the node that was deleted.
    pub node_id: Fs::NodeId,

    /// The path to the node at the time of its deletion.
    pub node_path: AbsPathBuf,

    /// TODO: docs.
    pub deletion_root_id: Fs::NodeId,
}

/// TODO: docs.
pub struct NodeMove<Fs: fs::Fs> {
    /// The ID of the node that was moved.
    pub node_id: Fs::NodeId,

    /// The path to the node before it was moved.
    pub old_path: AbsPathBuf,

    /// The path to the node after it was moved.
    pub new_path: AbsPathBuf,

    /// TODO: docs.
    pub move_root_id: Fs::NodeId,
}

/// TODO: docs.
#[derive(
    cauchy::Debug,
    derive_more::Display,
    cauchy::Error,
    cauchy::PartialEq,
    cauchy::Eq,
)]
#[display("{_0}")]
pub enum ReplicateError<Dst: Fs, Src: Fs> {
    /// TODO: docs.
    CreateDirectory(<Dst::Directory as Directory>::CreateDirectoryError),

    /// TODO: docs.
    CreateFile(<Dst::Directory as fs::Directory>::CreateFileError),

    /// TODO: docs.
    CreateSymlink(<Dst::Directory as fs::Directory>::CreateSymlinkError),

    /// TODO: docs.
    ReadDirectory(<Src::Directory as Directory>::ReadError),

    /// TODO: docs.
    ReadFile(<Src::File as fs::File>::ReadError),

    /// TODO: docs.
    ReadSymlink(<Src::Symlink as fs::Symlink>::ReadError),

    /// TODO: docs.
    WriteFile(<Dst::File as fs::File>::WriteError),
}
