use std::ffi::OsString;
use std::time::SystemTime;

use abs_path::NodeName;

use crate::{Inode, RealFs};

/// TODO: docs.
#[derive(Debug)]
pub struct Metadata {
    pub(crate) inner: async_fs::Metadata,
    pub(crate) node_kind: fs::NodeKind,
    pub(crate) node_name: OsString,
}

impl fs::Metadata for Metadata {
    type Fs = RealFs;

    #[inline]
    fn byte_len(&self) -> usize {
        self.inner.len() as usize
    }

    #[inline]
    fn created_at(&self) -> Option<SystemTime> {
        self.inner.created().ok()
    }

    #[inline]
    fn id(&self) -> Inode {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            self.inner.ino()
        }
    }

    #[inline]
    fn last_modified_at(&self) -> Option<SystemTime> {
        self.inner.modified().ok()
    }

    #[inline]
    fn name(&self) -> Result<&NodeName, fs::MetadataNameError> {
        self.node_name
            .to_str()
            .ok_or_else(|| {
                fs::MetadataNameError::NotUtf8(self.node_name.clone())
            })?
            .try_into()
            .map_err(fs::MetadataNameError::Invalid)
    }

    #[inline]
    fn node_kind(&self) -> fs::NodeKind {
        self.node_kind
    }
}
