use core::error::Error;

use abs_path::AbsPathBuf;
use futures_lite::Stream;

use super::FsNode;
use crate::fs::{self, AbsPath, Fs, Metadata, NodeName};

/// TODO: docs.
pub trait Directory: Sized {
    /// TODO: docs.
    type EventStream: Stream<Item = DirectoryEvent<Self>> + Unpin;

    /// TODO: docs.
    type Fs: Fs;

    /// TODO: docs.
    type Metadata: Metadata<Timestamp = <Self::Fs as Fs>::Timestamp>;

    /// TODO: docs.
    type CreateDirectoryError: Error;

    /// TODO: docs.
    type CreateFileError: Error;

    /// TODO: docs.
    type ClearError: Error;

    /// TODO: docs.
    type DeleteError: Error;

    /// TODO: docs.
    type ReadEntryError: Error;

    /// TODO: docs.
    type ReadError: Error;

    /// TODO: docs.
    fn create_directory(
        &self,
        directory_name: &NodeName,
    ) -> impl Future<Output = Result<Self, Self::CreateDirectoryError>>;

    /// TODO: docs.
    fn create_file(
        &self,
        file_name: &NodeName,
    ) -> impl Future<Output = Result<<Self::Fs as Fs>::File, Self::CreateFileError>>;

    /// TODO: docs.
    fn clear(&self) -> impl Future<Output = Result<(), Self::ClearError>>;

    /// TODO: docs.
    fn delete(self) -> impl Future<Output = Result<(), Self::DeleteError>>;

    /// TODO: docs.
    #[inline]
    fn name(&self) -> Option<&NodeName> {
        self.path().node_name()
    }

    /// TODO: docs.
    fn parent(
        &self,
    ) -> impl Future<Output = Option<<Self::Fs as Fs>::Directory>>;

    /// TODO: docs.
    fn path(&self) -> &AbsPath;

    /// TODO: docs.
    fn read(
        &self,
    ) -> impl Future<
        Output = Result<
            impl Stream<Item = Result<Self::Metadata, Self::ReadEntryError>>
            + use<Self>,
            Self::ReadError,
        >,
    >;

    /// TODO: docs.
    fn watch(&self) -> impl Future<Output = Self::EventStream>;
}

/// TODO: docs.
pub enum DirectoryEvent<Dir: Directory> {
    /// TODO: docs.
    Creation(ChildCreation<Dir::Fs>),

    /// TODO: docs.
    Deletion(DirectoryDeletion),

    /// TODO: docs.
    Move(DirectoryMove<Dir>),
}

/// TODO: docs.
pub struct ChildCreation<Fs: fs::Fs> {
    /// TODO: docs.
    pub child: FsNode<Fs>,

    /// TODO: docs.
    pub parent: Fs::Directory,
}

/// TODO: docs.
pub struct DirectoryDeletion {
    /// TODO: docs.
    pub dir_path: AbsPathBuf,
}

/// TODO: docs.
pub struct DirectoryMove<Dir: Directory> {
    /// TODO: docs.
    pub dir: Dir,

    /// TODO: docs.
    pub old_path: AbsPathBuf,
}
