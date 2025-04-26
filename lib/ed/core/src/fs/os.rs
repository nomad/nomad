//! TODO: docs.

use core::cell::RefCell;
use core::convert::Infallible;
use core::pin::Pin;
use core::task::{Context, Poll, ready};
use std::collections::VecDeque;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;

use futures_util::stream::{self, Stream, StreamExt};
use futures_util::{AsyncWriteExt, select};
use notify::{RecursiveMode, Watcher};

use crate::ByteOffset;
use crate::fs::{
    AbsPath,
    AbsPathBuf,
    Directory,
    DirectoryEvent,
    File,
    FileEvent,
    Fs,
    FsEvent,
    FsNode,
    Metadata,
    MetadataNameError,
    NodeKind,
    NodeName,
    Symlink,
};

/// TODO: docs.
pub type Inode = u64;

/// TODO: docs.
#[derive(Debug, Default, Copy, Clone)]
pub struct OsFs {}

/// TODO: docs.
pub struct OsDirectory {
    metadata: LazyOsMetadata,
}

/// TODO: docs.
pub struct OsFile {
    file: Option<async_fs::File>,
    metadata: LazyOsMetadata,
}

/// TODO: docs.
pub struct OsSymlink {
    metadata: async_fs::Metadata,
    path: AbsPathBuf,
}

/// TODO: docs.
pub struct OsMetadata {
    metadata: async_fs::Metadata,
    node_kind: NodeKind,
    node_name: OsString,
}

pin_project_lite::pin_project! {
    /// TODO: docs.
    pub struct OsWatcher {
        buffered: VecDeque<FsEvent<SystemTime>>,
        #[pin]
        inner: flume::r#async::RecvStream<
            'static,
            Result<(notify::Event, SystemTime), notify::Error>,
        >,
    }
}

struct LazyOsMetadata {
    metadata: RefCell<Option<async_fs::Metadata>>,
    path: AbsPathBuf,
}

impl OsFile {
    #[inline]
    fn open_options() -> async_fs::OpenOptions {
        let mut opts = async_fs::OpenOptions::new();
        opts.read(true).write(true);
        opts
    }

    #[inline]
    async fn with_file_async<R>(
        &mut self,
        fun: impl AsyncFnOnce(&mut async_fs::File) -> R,
    ) -> Result<R, io::Error> {
        loop {
            match &mut self.file {
                Some(file) => break Ok(fun(file).await),
                None => {
                    self.file =
                        Some(Self::open_options().open(self.path()).await?);
                },
            }
        }
    }
}

impl LazyOsMetadata {
    #[inline]
    fn lazy(path: AbsPathBuf) -> Self {
        Self { metadata: RefCell::new(None), path }
    }

    #[inline]
    fn new(metadata: async_fs::Metadata, path: AbsPathBuf) -> Self {
        Self { metadata: RefCell::new(Some(metadata)), path }
    }

    #[inline]
    async fn with<R>(
        &self,
        fun: impl FnOnce(&async_fs::Metadata) -> R,
    ) -> Result<R, io::Error> {
        if let Some(meta) = &*self.metadata.borrow() {
            return Ok(fun(meta));
        }
        let metadata = async_fs::metadata(&*self.path).await?;
        *self.metadata.borrow_mut() = Some(metadata);
        Ok(fun(self.metadata.borrow().as_ref().expect("just set it")))
    }
}

impl Fs for OsFs {
    type Directory = OsDirectory;
    type File = OsFile;
    type Symlink = OsSymlink;
    type Metadata = OsMetadata;
    type NodeId = Inode;
    type Timestamp = SystemTime;
    type Watcher = OsWatcher;

    type CreateDirectoryError = io::Error;
    type CreateFileError = io::Error;
    type NodeAtPathError = io::Error;
    type WatchError = notify::Error;

    #[inline]
    async fn create_directory<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> Result<Self::Directory, Self::CreateDirectoryError> {
        let path = path.as_ref();
        async_fs::create_dir(path).await?;
        Ok(Self::Directory { metadata: LazyOsMetadata::lazy(path.to_owned()) })
    }

    #[inline]
    async fn create_file<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> Result<Self::File, Self::CreateFileError> {
        let path = path.as_ref();
        let file = OsFile::open_options().create_new(true).open(path).await?;
        Ok(Self::File {
            file: file.into(),
            metadata: LazyOsMetadata::lazy(path.to_owned()),
        })
    }

    #[inline]
    async fn node_at_path<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> Result<Option<FsNode<Self>>, Self::NodeAtPathError> {
        let path = path.as_ref();
        let metadata = match async_fs::symlink_metadata(path).await {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let Ok(file_type) = metadata.file_type().try_into() else {
            return Ok(None);
        };
        Ok(Some(match file_type {
            NodeKind::File => FsNode::File(OsFile {
                file: None,
                metadata: LazyOsMetadata::new(metadata, path.to_owned()),
            }),
            NodeKind::Directory => FsNode::Directory(OsDirectory {
                metadata: LazyOsMetadata::new(metadata, path.to_owned()),
            }),
            NodeKind::Symlink => {
                FsNode::Symlink(OsSymlink { metadata, path: path.to_owned() })
            },
        }))
    }

    #[inline]
    fn now(&self) -> Self::Timestamp {
        SystemTime::now()
    }

    #[inline]
    async fn watch<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> Result<Self::Watcher, Self::WatchError> {
        let (tx, rx) = flume::unbounded();
        let mut watcher = notify::recommended_watcher(
            move |event_res: Result<_, notify::Error>| {
                let _ =
                    tx.send(event_res.map(|event| (event, SystemTime::now())));
            },
        )?;
        watcher.watch(
            std::path::Path::new(path.as_ref().as_str()),
            RecursiveMode::Recursive,
        )?;
        Ok(OsWatcher {
            buffered: VecDeque::default(),
            inner: rx.into_stream(),
        })
    }
}

impl Directory for OsDirectory {
    type EventStream = futures_util::stream::Pending<DirectoryEvent<OsFs>>;
    type Fs = OsFs;

    type ClearError = io::Error;
    type CreateDirectoryError = io::Error;
    type CreateFileError = io::Error;
    type DeleteError = io::Error;
    type MetadataError = io::Error;
    type ReadEntryError = io::Error;
    type ReadError = io::Error;

    #[inline]
    async fn create_directory(
        &self,
        directory_name: &NodeName,
    ) -> Result<Self, Self::CreateDirectoryError> {
        OsFs::default()
            .create_directory(self.path().join(directory_name))
            .await
    }

    #[inline]
    async fn create_file(
        &self,
        file_name: &NodeName,
    ) -> Result<OsFile, Self::CreateFileError> {
        OsFs::default().create_file(self.path().join(file_name)).await
    }

    #[inline]
    async fn clear(&self) -> Result<(), Self::ClearError> {
        async_fs::remove_dir_all(self.path()).await?;
        async_fs::create_dir(self.path()).await?;
        Ok(())
    }

    #[inline]
    async fn delete(self) -> Result<(), Self::DeleteError> {
        async_fs::remove_dir_all(self.path()).await
    }

    #[inline]
    fn id(&self) -> Inode {
        todo!()
    }

    #[inline]
    async fn meta(&self) -> Result<OsMetadata, Self::MetadataError> {
        self.metadata
            .with(|inner| OsMetadata {
                metadata: inner.clone(),
                node_kind: NodeKind::Directory,
                node_name: self
                    .name()
                    .map(|n| n.as_str().into())
                    .unwrap_or_default(),
            })
            .await
    }

    #[inline]
    async fn parent(&self) -> Option<Self> {
        self.path().parent().map(|parent| Self {
            metadata: LazyOsMetadata::lazy(parent.to_owned()),
        })
    }

    #[inline]
    fn path(&self) -> &AbsPath {
        &self.metadata.path
    }

    #[inline]
    async fn read(
        &self,
    ) -> Result<
        impl Stream<Item = Result<OsMetadata, Self::ReadEntryError>> + use<>,
        Self::ReadError,
    > {
        let read_dir = async_fs::read_dir(self.path()).await?.fuse();
        let get_metadata = stream::FuturesUnordered::new();
        Ok(stream::unfold(
            (read_dir, get_metadata, self.path().to_owned()),
            move |(mut read_dir, mut get_metadata, dir_path)| async move {
                let metadata_res = loop {
                    select! {
                        res = read_dir.select_next_some() => {
                            let dir_entry = match res {
                                Ok(entry) => entry,
                                Err(err) => break Err(err),
                            };
                            let dir_path = dir_path.clone();
                            get_metadata.push(async move {
                                let node_name = dir_entry.file_name();
                                let entry_path =
                                    PathBuf::from(dir_path.as_str())
                                        .join(&node_name);
                                let meta =
                                    async_fs::symlink_metadata(entry_path)
                                        .await?;
                                Ok((meta, node_name))
                            });
                        },
                        res = get_metadata.select_next_some() => {
                            let (metadata, node_name) = match res {
                                Ok(tuple) => tuple,
                                Err(err) => break Err(err),
                            };
                            let file_type = metadata.file_type();
                            let node_kind = if file_type.is_dir() {
                                NodeKind::Directory
                            } else if file_type.is_file() {
                                NodeKind::File
                            } else if file_type.is_symlink() {
                                NodeKind::Symlink
                            } else {
                                continue
                            };
                            break Ok(OsMetadata {
                                metadata,
                                node_kind,
                                node_name,
                            })
                        },
                        complete => return None,
                    }
                };
                Some((metadata_res, (read_dir, get_metadata, dir_path)))
            },
        ))
    }

    #[inline]
    fn watch(&self) -> Self::EventStream {
        todo!()
    }
}

impl File for OsFile {
    type EventStream = futures_util::stream::Pending<FileEvent<OsFs>>;
    type Fs = OsFs;

    type Error = io::Error;
    type DeleteError = io::Error;
    type MetadataError = io::Error;
    type ReadError = io::Error;
    type WriteError = io::Error;

    #[inline]
    async fn byte_len(&self) -> Result<ByteOffset, Self::Error> {
        self.metadata.with(|meta| meta.len().into()).await
    }

    #[inline]
    async fn delete(self) -> Result<(), Self::DeleteError> {
        async_fs::remove_file(self.path()).await
    }

    #[inline]
    fn id(&self) -> Inode {
        todo!()
    }

    #[inline]
    async fn meta(&self) -> Result<OsMetadata, Self::MetadataError> {
        self.metadata
            .with(|inner| OsMetadata {
                metadata: inner.clone(),
                node_kind: NodeKind::File,
                node_name: self.name().as_str().into(),
            })
            .await
    }

    #[inline]
    async fn parent(&self) -> <Self::Fs as Fs>::Directory {
        OsDirectory {
            metadata: LazyOsMetadata::lazy(
                self.path().parent().expect("has a parent").to_owned(),
            ),
        }
    }

    #[inline]
    fn path(&self) -> &AbsPath {
        &self.metadata.path
    }

    #[inline]
    async fn read(&self) -> Result<Vec<u8>, Self::ReadError> {
        async_fs::read(self.path()).await
    }

    #[inline]
    fn watch(&self) -> Self::EventStream {
        todo!()
    }

    #[inline]
    async fn write<C: AsRef<[u8]>>(
        &mut self,
        new_contents: C,
    ) -> Result<(), Self::WriteError> {
        self.with_file_async(async move |file| {
            file.write_all(new_contents.as_ref()).await
        })
        .await?
    }
}

impl Symlink for OsSymlink {
    type Fs = OsFs;

    type DeleteError = io::Error;
    type FollowError = io::Error;
    type MetadataError = Infallible;
    type ReadError = io::Error;

    #[inline]
    async fn delete(self) -> Result<(), Self::DeleteError> {
        async_fs::remove_file(self.path).await
    }

    #[inline]
    async fn follow(&self) -> Result<Option<FsNode<OsFs>>, Self::FollowError> {
        let target_path = async_fs::read_link(&*self.path).await?;
        let path = <&AbsPath>::try_from(&*target_path)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        OsFs::default().node_at_path(path).await
    }

    #[inline]
    async fn follow_recursively(
        &self,
    ) -> Result<Option<FsNode<OsFs>>, Self::FollowError> {
        let target_path = async_fs::canonicalize(&*self.path).await?;
        let path = <&AbsPath>::try_from(&*target_path)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        OsFs::default().node_at_path(path).await
    }

    #[inline]
    fn id(&self) -> Inode {
        todo!()
    }

    #[inline]
    async fn meta(&self) -> Result<OsMetadata, Self::MetadataError> {
        Ok(OsMetadata {
            metadata: self.metadata.clone(),
            node_kind: NodeKind::Symlink,
            node_name: self.name().as_str().into(),
        })
    }

    #[inline]
    fn path(&self) -> &AbsPath {
        &self.path
    }

    #[inline]
    async fn read_path(&self) -> Result<String, Self::ReadError> {
        async_fs::read_link(&*self.path)
            .await
            .map(|path| path.display().to_string())
    }
}

impl Stream for OsWatcher {
    type Item = Result<FsEvent<SystemTime>, notify::Error>;

    #[inline]
    fn poll_next(
        self: Pin<&mut Self>,
        ctx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        loop {
            if let Some(event) = this.buffered.pop_front() {
                return Poll::Ready(Some(Ok(event)));
            }
            let Some((event, timestamp)) =
                ready!(this.inner.as_mut().poll_next(ctx)).transpose()?
            else {
                return Poll::Ready(None);
            };
            this.buffered.extend(FsEvent::from_notify(event, timestamp));
        }
    }
}

impl Metadata for OsMetadata {
    type Fs = OsFs;

    #[inline]
    fn byte_len(&self) -> ByteOffset {
        self.metadata.len().into()
    }

    #[inline]
    fn created_at(&self) -> Option<SystemTime> {
        self.metadata.created().ok()
    }

    #[inline]
    fn last_modified_at(&self) -> Option<SystemTime> {
        self.metadata.modified().ok()
    }

    #[inline]
    fn name(&self) -> Result<&NodeName, MetadataNameError> {
        self.node_name
            .to_str()
            .ok_or_else(|| {
                MetadataNameError::NotUtf8(Some(self.node_name.clone()))
            })?
            .try_into()
            .map_err(MetadataNameError::Invalid)
    }

    #[inline]
    fn node_kind(&self) -> NodeKind {
        self.node_kind
    }
}
