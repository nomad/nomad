use core::error::Error;
use core::fmt;
use std::ffi::OsString;

use abs_path::{InvalidNodeNameError, NodeName};

use crate::ByteOffset;
use crate::fs::{Fs, NodeKind};

/// TODO: docs.
pub trait Metadata: Send + Sync {
    /// TODO: docs.
    type Fs: Fs;

    /// TODO: docs.
    fn byte_len(&self) -> ByteOffset;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataNameError {
    /// TODO: docs.
    Invalid(InvalidNodeNameError),

    /// TODO: docs.
    NotUtf8(Option<OsString>),

    /// TODO: docs.
    MetadataIsForRoot,
}

impl fmt::Display for MetadataNameError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(err) => fmt::Display::fmt(err, f),
            Self::NotUtf8(maybe_os_str) => {
                f.write_str("file name ")?;
                if let Some(os_str) = maybe_os_str {
                    fmt::Debug::fmt(os_str, f)?;
                }
                f.write_str("is not valid UTF-8")
            },
            Self::MetadataIsForRoot => f.write_str(
                "metadata is for the root directory, which doesn't have a \
                 name",
            ),
        }
    }
}

impl Error for MetadataNameError {}
