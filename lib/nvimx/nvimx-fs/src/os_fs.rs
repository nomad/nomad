//! TODO: docs.

use alloc::borrow::Cow;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::ffi::OsString;
use std::io;

use futures_util::{ready, Stream};

use crate::{
    AbsPath,
    DirEntry,
    Fs,
    FsNode,
    FsNodeKind,
    FsNodeName,
    InvalidFsNodeNameError,
};

/// TODO: docs.
#[derive(Debug, Default, Copy, Clone)]
pub struct OsFs;

/// TODO: docs.
pub struct OsDirEntry {
    inner: async_fs::DirEntry,
}

/// TODO: docs.
pub struct OsDirectory<Path> {
    metadata: async_fs::Metadata,
    path: Path,
}

/// TODO: docs.
pub struct OsFile<Path> {
    metadata: async_fs::Metadata,
    path: Path,
}

pin_project_lite::pin_project! {
    /// TODO: docs.
    pub struct OsReadDir {
        #[pin]
        inner: async_fs::ReadDir,
    }
}

/// TODO: docs.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum OsNameError {
    /// TODO: docs.
    #[error("file name {:?} is not valid UTF-8", .0)]
    NotUtf8(OsString),

    /// TODO: docs.
    #[error(transparent)]
    Invalid(#[from] InvalidFsNodeNameError),
}

impl Fs for OsFs {
    type DirEntry = OsDirEntry;
    type Directory<Path> = OsDirectory<Path>;
    type File<Path> = OsFile<Path>;
    type ReadDir = OsReadDir;
    type DirEntryError = io::Error;
    type NodeAtPathError = io::Error;
    type ReadDirError = io::Error;

    async fn node_at_path<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> Result<Option<FsNode<Self, P>>, Self::NodeAtPathError> {
        let metadata = match async_fs::metadata(path.as_ref()).await {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        Ok(Some(match metadata.file_type().into() {
            FsNodeKind::File => FsNode::File(OsFile { metadata, path }),
            FsNodeKind::Directory => {
                FsNode::Directory(OsDirectory { metadata, path })
            },
            FsNodeKind::Symlink => todo!("can't handle symlinks yet"),
        }))
    }

    async fn read_dir<P: AsRef<AbsPath>>(
        &self,
        dir_path: P,
    ) -> Result<Self::ReadDir, Self::ReadDirError> {
        async_fs::read_dir(dir_path.as_ref())
            .await
            .map(|inner| OsReadDir { inner })
    }
}

impl Stream for OsReadDir {
    type Item = Result<OsDirEntry, io::Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        ctx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        match ready!(self.project().inner.poll_next(ctx)) {
            Some(Ok(entry)) => {
                Poll::Ready(Some(Ok(OsDirEntry { inner: entry })))
            },
            Some(Err(err)) => Poll::Ready(Some(Err(err))),
            None => Poll::Ready(None),
        }
    }
}

impl DirEntry for OsDirEntry {
    type NameError = OsNameError;
    type NodeKindError = io::Error;

    async fn name(&self) -> Result<Cow<'_, FsNodeName>, Self::NameError> {
        let os_name = self.inner.file_name();
        let fs_name: &FsNodeName = os_name
            .to_str()
            .ok_or_else(|| OsNameError::NotUtf8(os_name.clone()))?
            .try_into()?;
        Ok(Cow::Owned(fs_name.to_owned()))
    }

    async fn node_kind(&self) -> Result<FsNodeKind, Self::NodeKindError> {
        self.inner.file_type().await.map(Into::into)
    }
}
