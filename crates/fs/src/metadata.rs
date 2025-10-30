use core::fmt;
use std::ffi::OsString;

use abs_path::{InvalidNodeNameError, NodeName};

use crate::{Fs, NodeKind};

/// TODO: docs.
pub trait Metadata: fmt::Debug + Send + Sync {
    /// TODO: docs.
    type Fs: Fs;

    /// TODO: docs.
    fn byte_len(&self) -> usize;

    /// TODO: docs.
    fn created_at(&self) -> Option<<Self::Fs as Fs>::Timestamp>;

    /// TODO: docs.
    fn id(&self) -> <Self::Fs as Fs>::NodeId;

    /// TODO: docs.
    fn last_modified_at(&self) -> Option<<Self::Fs as Fs>::Timestamp>;

    /// TODO: docs.
    fn name(&self) -> Result<&NodeName, MetadataNameError>;

    /// TODO: docs.
    fn node_kind(&self) -> NodeKind;
}

/// TODO: docs.
#[derive(Debug, Clone, derive_more::Display, PartialEq, Eq, cauchy::Error)]
pub enum MetadataNameError {
    /// TODO: docs.
    #[display("{_0}")]
    Invalid(InvalidNodeNameError),

    /// TODO: docs.
    #[display("file name {_0:?} is not valid UTF-8")]
    NotUtf8(OsString),

    /// TODO: docs.
    #[display("metadata is for the root directory, which doesn't have a name")]
    MetadataIsForRoot,
}
