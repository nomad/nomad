//! TODO: docs.

use core::pin::Pin;
use core::task::{Context, Poll, ready};
use std::collections::VecDeque;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;

use futures_util::stream::{self, Stream, StreamExt};
use futures_util::{AsyncWriteExt, select};

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
    SymlinkEvent,
};

/// TODO: docs.
pub type Inode = u64;

/// TODO: docs.
#[derive(Debug, Default, Copy, Clone)]
pub struct OsFs {}

/// TODO: docs.
pub struct OsDirectory {
    metadata: async_fs::Metadata,
    path: AbsPathBuf,
}

/// TODO: docs.
pub struct OsFile {
    file: Option<async_fs::File>,
    metadata: async_fs::Metadata,
    path: AbsPathBuf,
}

/// TODO: docs.
pub struct OsSymlink {
    metadata: async_fs::Metadata,
    path: AbsPathBuf,
}

/// TODO: docs.
pub struct OsMetadata {
    inner: async_fs::Metadata,
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

impl Fs for OsFs {
    type Directory = OsDirectory;
    type File = OsFile;
    type Symlink = OsSymlink;
    type Metadata = OsMetadata;
    type NodeId = Inode;
    type Timestamp = SystemTime;

    type CreateDirectoriesError = io::Error;
    type NodeAtPathError = io::Error;

    #[inline]
    async fn create_all_missing_directories<P: AsRef<AbsPath> + Send>(
        &self,
        path: P,
    ) -> Result<Self::Directory, Self::CreateDirectoriesError> {
        let path = path.as_ref();
        async_fs::create_dir_all(path).await?;
        let metadata = async_fs::metadata(path).await?;
        Ok(OsDirectory { path: path.to_owned(), metadata })
    }

    #[inline]
    async fn node_at_path<P: AsRef<AbsPath> + Send>(
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
                metadata,
                path: path.to_owned(),
            }),
            NodeKind::Directory => FsNode::Directory(OsDirectory {
                metadata,
                path: path.to_owned(),
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
}

impl Directory for OsDirectory {
    type EventStream = futures_util::stream::Pending<DirectoryEvent<OsFs>>;
    type Fs = OsFs;

    type ClearError = io::Error;
    type CreateDirectoryError = io::Error;
    type CreateFileError = io::Error;
    type CreateSymlinkError = io::Error;
    type DeleteError = io::Error;
    type ListError = io::Error;
    type ParentError = io::Error;
    type ReadMetadataError = io::Error;

    #[inline]
    async fn create_directory(
        &self,
        directory_name: &NodeName,
    ) -> Result<Self, Self::CreateDirectoryError> {
        let path = self.path.clone().join(directory_name);
        async_fs::create_dir(&path).await?;
        let metadata = async_fs::metadata(&path).await?;
        Ok(Self { metadata, path })
    }

    #[inline]
    async fn create_file(
        &self,
        file_name: &NodeName,
    ) -> Result<OsFile, Self::CreateFileError> {
        let path = self.path.clone().join(file_name);
        let file = OsFile::open_options().create_new(true).open(&path).await?;
        let metadata = file.metadata().await?;
        Ok(OsFile { file: file.into(), metadata, path })
    }

    #[inline]
    async fn create_symlink(
        &self,
        symlink_name: &NodeName,
        target_path: &str,
    ) -> Result<OsSymlink, Self::CreateSymlinkError> {
        #[cfg(unix)]
        {
            let path = self.path.clone().join(symlink_name);
            async_fs::unix::symlink(target_path, &path).await?;
            let metadata = async_fs::metadata(&path).await?;
            Ok(OsSymlink { metadata, path })
        }
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
    async fn list_metas(
        &self,
    ) -> Result<
        impl Stream<Item = Result<OsMetadata, Self::ReadMetadataError>> + use<>,
        Self::ListError,
    > {
        let read_dir = async_fs::read_dir(self.path()).await?.fuse();
        let get_metadata = stream::FuturesUnordered::new();
        Ok(Box::pin(stream::unfold(
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
                                inner: metadata,
                                node_kind,
                                node_name,
                            })
                        },
                        complete => return None,
                    }
                };
                Some((metadata_res, (read_dir, get_metadata, dir_path)))
            },
        )))
    }

    #[inline]
    fn meta(&self) -> OsMetadata {
        OsMetadata {
            inner: self.metadata.clone(),
            node_kind: NodeKind::Directory,
            node_name: self
                .name()
                .map(|n| n.as_str().into())
                .unwrap_or_default(),
        }
    }

    #[inline]
    async fn parent(&self) -> Result<Option<Self>, Self::ParentError> {
        let Some(parent_path) = self.path().parent() else { return Ok(None) };
        let metadata = async_fs::metadata(parent_path).await?;
        Ok(Some(OsDirectory { path: parent_path.to_owned(), metadata }))
    }

    #[inline]
    fn path(&self) -> &AbsPath {
        &self.path
    }

    #[inline]
    fn watch(&self) -> Self::EventStream {
        todo!()
    }
}

impl File for OsFile {
    type EventStream = futures_util::stream::Pending<FileEvent<OsFs>>;
    type Fs = OsFs;

    type DeleteError = io::Error;
    type ParentError = io::Error;
    type ReadError = io::Error;
    type WriteError = io::Error;

    #[inline]
    async fn delete(self) -> Result<(), Self::DeleteError> {
        async_fs::remove_file(self.path()).await
    }

    #[inline]
    fn meta(&self) -> OsMetadata {
        OsMetadata {
            inner: self.metadata.clone(),
            node_kind: NodeKind::File,
            node_name: self.name().as_str().into(),
        }
    }

    #[inline]
    async fn parent(&self) -> Result<OsDirectory, Self::ParentError> {
        let parent_path = self.path().parent().expect("has a parent");
        let metadata = async_fs::metadata(parent_path).await?;
        Ok(OsDirectory { path: parent_path.to_owned(), metadata })
    }

    #[inline]
    fn path(&self) -> &AbsPath {
        &self.path
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
    async fn write<C: AsRef<[u8]> + Send>(
        &mut self,
        new_contents: C,
    ) -> Result<(), Self::WriteError> {
        self.with_file_async(async move |file| {
            file.write_all(new_contents.as_ref()).await?;
            file.sync_all().await?;
            Ok(())
        })
        .await?
    }
}

impl Symlink for OsSymlink {
    type EventStream = futures_util::stream::Pending<SymlinkEvent<OsFs>>;
    type Fs = OsFs;

    type DeleteError = io::Error;
    type FollowError = io::Error;
    type ParentError = io::Error;
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
    fn meta(&self) -> OsMetadata {
        OsMetadata {
            inner: self.metadata.clone(),
            node_kind: NodeKind::Symlink,
            node_name: self.name().as_str().into(),
        }
    }

    #[inline]
    async fn parent(&self) -> Result<OsDirectory, Self::ParentError> {
        let parent_path = self.path().parent().expect("has a parent");
        let metadata = async_fs::metadata(parent_path).await?;
        Ok(OsDirectory { path: parent_path.to_owned(), metadata })
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

    #[inline]
    fn watch(&self) -> Self::EventStream {
        todo!()
    }
}

impl Metadata for OsMetadata {
    type Fs = OsFs;

    #[inline]
    fn byte_len(&self) -> ByteOffset {
        self.inner.len().into()
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
    fn name(&self) -> Result<&NodeName, MetadataNameError> {
        self.node_name
            .to_str()
            .ok_or_else(|| MetadataNameError::NotUtf8(self.node_name.clone()))?
            .try_into()
            .map_err(MetadataNameError::Invalid)
    }

    #[inline]
    fn node_kind(&self) -> NodeKind {
        self.node_kind
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

impl AsRef<OsFs> for OsDirectory {
    fn as_ref(&self) -> &OsFs {
        &OsFs {}
    }
}
