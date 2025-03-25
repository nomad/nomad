use core::error::Error;

use crate::ByteOffset;
use crate::fs::{Fs, FsNodeKind, NodeNameBuf};

/// TODO: docs.
pub trait Metadata {
    /// TODO: docs.
    type Fs: Fs;

    /// TODO: docs.
    type NameError: Error;

    /// TODO: docs.
    type NodeKindError: Error;

    /// TODO: docs.
    fn byte_len(&self) -> ByteOffset;

    /// TODO: docs.
    fn created_at(&self) -> Option<<Self::Fs as Fs>::Timestamp>;

    /// TODO: docs.
    fn last_modified_at(&self) -> Option<<Self::Fs as Fs>::Timestamp>;

    /// TODO: docs.
    fn name(
        &self,
    ) -> impl Future<Output = Result<NodeNameBuf, Self::NameError>>;

    /// TODO: docs.
    fn node_kind(
        &self,
    ) -> impl Future<Output = Result<FsNodeKind, Self::NodeKindError>>;
}
