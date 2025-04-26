use core::error::Error;

use futures_util::Stream;

use crate::ByteOffset;
use crate::fs::{self, AbsPath, Fs, NodeDeletion, NodeMove, NodeName};

/// TODO: docs.
pub trait File: Send {
    /// TODO: docs.
    type EventStream: Stream<Item = FileEvent<Self::Fs>> + Unpin;

    /// TODO: docs.
    type Fs: Fs;

    /// TODO: docs.
    type Error: Error + Send;

    /// TODO: docs.
    type DeleteError: Error + Send;

    /// TODO: docs.
    type MetadataError: Error + Send;

    /// TODO: docs.
    type ReadError: Error + Send;

    /// TODO: docs.
    type WriteError: Error + Send;

    /// TODO: docs.
    fn byte_len(
        &self,
    ) -> impl Future<Output = Result<ByteOffset, Self::Error>>;

    /// TODO: docs.
    fn delete(self) -> impl Future<Output = Result<(), Self::DeleteError>>;

    /// TODO: docs.
    fn id(&self) -> <Self::Fs as Fs>::NodeId;

    /// TODO: docs.
    fn meta(
        &self,
    ) -> impl Future<Output = Result<<Self::Fs as Fs>::Metadata, Self::MetadataError>>;

    /// TODO: docs.
    #[inline]
    fn name(&self) -> &NodeName {
        self.path().node_name().expect("path is not root")
    }

    /// TODO: docs.
    fn parent(&self) -> impl Future<Output = <Self::Fs as Fs>::Directory>;

    /// TODO: docs.
    fn path(&self) -> &AbsPath;

    /// TODO: docs.
    fn read(&self) -> impl Future<Output = Result<Vec<u8>, Self::ReadError>>;

    /// TODO: docs.
    fn watch(&self) -> Self::EventStream;

    /// TODO: docs.
    fn write<C: AsRef<[u8]>>(
        &mut self,
        new_contents: C,
    ) -> impl Future<Output = Result<(), Self::WriteError>>;
}

/// TODO: docs.
pub enum FileEvent<Fs: fs::Fs> {
    /// TODO: docs.
    Deletion(NodeDeletion<Fs>),

    /// TODO: docs.
    Move(NodeMove<Fs>),

    /// TODO: docs.
    Modification(FileModification<Fs>),
}

/// TODO: docs.
pub struct FileModification<Fs: fs::Fs> {
    /// The node ID of the file.
    pub file_id: Fs::NodeId,

    /// TODO: docs.
    pub modified_at: Fs::Timestamp,
}
