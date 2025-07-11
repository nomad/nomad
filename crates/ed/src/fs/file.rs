use core::error::Error;

use abs_path::{AbsPath, NodeName};
use futures_util::Stream;

use crate::ByteOffset;
use crate::fs::{self, Fs};

/// TODO: docs.
pub trait File: Send + Sync {
    /// TODO: docs.
    type EventStream: Stream<Item = FileEvent<Self::Fs>> + Send + Unpin;

    /// TODO: docs.
    type Fs: Fs;

    /// TODO: docs.
    type DeleteError: Error + Send;

    /// TODO: docs.
    type MoveError: Error + Send;

    /// TODO: docs.
    type ParentError: Error + Send;

    /// TODO: docs.
    type ReadError: Error + Send;

    /// TODO: docs.
    type WriteError: Error + Send;

    /// TODO: docs.
    #[inline]
    fn byte_len(&self) -> ByteOffset {
        fs::Metadata::byte_len(&self.meta())
    }

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
    fn r#move(
        &self,
        new_path: &AbsPath,
    ) -> impl Future<Output = Result<(), Self::MoveError>> + Send;

    /// TODO: docs.
    #[inline]
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
    fn read(
        &self,
    ) -> impl Future<Output = Result<Vec<u8>, Self::ReadError>> + Send;

    /// TODO: docs.
    fn watch(&self) -> Self::EventStream;

    /// TODO: docs.
    fn write<C: AsRef<[u8]> + Send>(
        &mut self,
        new_contents: C,
    ) -> impl Future<Output = Result<(), Self::WriteError>> + Send;
}

/// TODO: docs.
#[derive(cauchy::Clone)]
pub enum FileEvent<Fs: fs::Fs> {
    /// TODO: docs.
    IdChange(FileIdChange<Fs>),

    /// TODO: docs.
    Modification(FileModification<Fs>),
}

/// TODO: docs.
#[derive(cauchy::Clone)]
pub struct FileModification<Fs: fs::Fs> {
    /// The node ID of the file.
    pub file_id: Fs::NodeId,

    /// TODO: docs.
    pub modified_at: Fs::Timestamp,
}

/// TODO: docs.
#[derive(cauchy::Clone)]
pub struct FileIdChange<Fs: fs::Fs> {
    /// The file's old node ID.
    pub old_id: Fs::NodeId,

    /// The file's new node ID.
    pub new_id: Fs::NodeId,
}
