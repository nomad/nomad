//! TODO: docs.

use core::pin::Pin;
use core::task::{Context, Poll};
use std::borrow::Cow;
use std::ffi::OsString;
use std::fs::Metadata;
use std::io;
use std::time::SystemTime;

use futures_lite::{Stream, ready};

use crate::fs::{
    AbsPath,
    AbsPathBuf,
    DirEntry,
    Fs,
    FsEvent,
    FsNode,
    FsNodeKind,
    FsNodeName,
    InvalidFsNodeNameError,
    Watcher,
};

/// TODO: docs.
#[derive(Debug, Default, Copy, Clone)]
pub struct OsFs {}

/// TODO: docs.
pub struct OsDirEntry {
    inner: async_fs::DirEntry,
}

/// TODO: docs.
pub struct OsDirectory<Path> {
    _metadata: async_fs::Metadata,
    _path: Path,
}

/// TODO: docs.
pub struct OsFile<Path> {
    _metadata: async_fs::Metadata,
    _path: Path,
}

pin_project_lite::pin_project! {
    /// TODO: docs.
    pub struct OsReadDir {
        #[pin]
        inner: async_fs::ReadDir,
    }
}

/// TODO: docs.
pub struct OsWatcher {
    watched_path: AbsPathBuf,
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
    type DirEntryError = io::Error;
    type Directory<Path> = OsDirectory<Path>;
    type File<Path> = OsFile<Path>;
    type NodeAtPathError = io::Error;
    type ReadDir = OsReadDir;
    type ReadDirError = io::Error;
    type Timestamp = SystemTime;
    type WatchError = core::convert::Infallible;
    type Watcher = OsWatcher;

    #[inline]
    async fn node_at_path<P: AsRef<AbsPath>>(
        &mut self,
        path: P,
    ) -> Result<Option<FsNode<Self, P>>, Self::NodeAtPathError> {
        let metadata = match async_fs::metadata(path.as_ref()).await {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        Ok(Some(match metadata.file_type().into() {
            FsNodeKind::File => {
                FsNode::File(OsFile { _metadata: metadata, _path: path })
            },
            FsNodeKind::Directory => FsNode::Directory(OsDirectory {
                _metadata: metadata,
                _path: path,
            }),
            FsNodeKind::Symlink => todo!("can't handle symlinks yet"),
        }))
    }

    #[inline]
    fn now(&self) -> Self::Timestamp {
        SystemTime::now()
    }

    #[inline]
    async fn read_dir<P: AsRef<AbsPath>>(
        &self,
        dir_path: P,
    ) -> Result<Self::ReadDir, Self::ReadDirError> {
        async_fs::read_dir(dir_path.as_ref())
            .await
            .map(|inner| OsReadDir { inner })
    }

    #[inline]
    async fn watch<P: AsRef<AbsPath>>(
        &mut self,
        path: P,
    ) -> Result<Self::Watcher, Self::WatchError> {
        Ok(OsWatcher { watched_path: path.as_ref().to_owned() })
    }
}

impl Stream for OsReadDir {
    type Item = Result<OsDirEntry, io::Error>;

    #[inline]
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
    type MetadataError = io::Error;
    type NameError = OsNameError;
    type NodeKindError = io::Error;

    #[inline]
    async fn metadata(&self) -> Result<Metadata, Self::MetadataError> {
        self.inner.metadata().await
    }

    #[inline]
    async fn name(&self) -> Result<Cow<'_, FsNodeName>, Self::NameError> {
        let os_name = self.inner.file_name();
        let fs_name: &FsNodeName = os_name
            .to_str()
            .ok_or_else(|| OsNameError::NotUtf8(os_name.clone()))?
            .try_into()?;
        Ok(Cow::Owned(fs_name.to_owned()))
    }

    #[inline]
    async fn node_kind(&self) -> Result<FsNodeKind, Self::NodeKindError> {
        self.inner.file_type().await.map(Into::into)
    }
}

impl Watcher<OsFs> for OsWatcher {
    type Error = core::convert::Infallible;

    #[inline]
    fn register_handler<F>(&mut self, _callback: F)
    where
        F: FnMut(Result<FsEvent<OsFs>, Self::Error>) -> bool + 'static,
    {
        todo!()
    }

    #[inline]
    fn watched_path(&self) -> &AbsPath {
        &self.watched_path
    }
}
