use core::ops::{Deref, DerefMut};
use std::io;

use abs_path::{AbsPath, AbsPathBuf};
use futures_util::{AsyncReadExt, AsyncWriteExt, stream};

use crate::file_descriptor_permit::FileDescriptorPermit;
use crate::{Directory, IoErrorExt, Metadata, RealFs};

/// TODO: docs.
pub struct File {
    inner: Option<FileInner>,
    metadata: async_fs::Metadata,
    path: AbsPathBuf,
}

struct FileInner {
    inner: async_fs::File,
    _fd_permit: FileDescriptorPermit,
}

impl File {
    #[inline]
    pub(crate) async fn create(path: AbsPathBuf) -> io::Result<Self> {
        let inner = FileInner::create(&path).await?;
        let metadata = inner.metadata().await?;
        Ok(Self { inner: Some(inner), metadata, path })
    }

    #[inline]
    pub(crate) fn new(metadata: async_fs::Metadata, path: AbsPathBuf) -> Self {
        Self { inner: None, metadata, path }
    }

    #[inline]
    async fn with_inner<R>(
        &mut self,
        fun: impl AsyncFnOnce(
            &mut FileInner,
            &mut async_fs::Metadata,
            &AbsPath,
        ) -> R,
    ) -> io::Result<R> {
        loop {
            match &mut self.inner {
                Some(inner) => {
                    break Ok(fun(inner, &mut self.metadata, &self.path).await);
                },
                None => {
                    self.inner = Some(FileInner::open(&self.path).await?);
                },
            }
        }
    }
}

impl FileInner {
    async fn create(path: &AbsPath) -> io::Result<Self> {
        Self::new(path, true).await
    }

    #[allow(clippy::disallowed_methods)]
    async fn new(path: &AbsPath, create_new: bool) -> io::Result<Self> {
        let _fd_permit = FileDescriptorPermit::acquire().await;

        let inner = async_fs::OpenOptions::new()
            .create_new(create_new)
            .read(true)
            .write(true)
            .open(path)
            .await
            .with_context(|| {
                format!(
                    "couldn't {verb} file at {path}",
                    verb = if create_new { "create" } else { "open" }
                )
            })?;

        Ok(Self { inner, _fd_permit })
    }

    async fn open(path: &AbsPath) -> io::Result<Self> {
        Self::new(path, false).await
    }
}

impl fs::File for File {
    type EventStream = stream::Pending<fs::FileEvent<RealFs>>;
    type Fs = RealFs;

    type DeleteError = io::Error;
    type MoveError = io::Error;
    type ParentError = io::Error;
    type ReadError = io::Error;
    type WriteError = io::Error;

    #[inline]
    async fn delete(self) -> Result<(), Self::DeleteError> {
        async_fs::remove_file(self.path()).await.with_context(|| {
            format!("couldn't delete file at {}", self.path())
        })
    }

    #[inline]
    fn meta(&self) -> Metadata {
        Metadata {
            inner: self.metadata.clone(),
            node_kind: fs::NodeKind::File,
            node_name: self.name().as_str().into(),
        }
    }

    #[inline]
    async fn r#move(&self, new_path: &AbsPath) -> Result<(), Self::MoveError> {
        crate::move_node(self.path(), new_path).await.with_context(|| {
            format!("couldn't move file at {} to {}", self.path(), new_path)
        })
    }

    #[inline]
    async fn parent(&self) -> Result<Directory, Self::ParentError> {
        let parent_path = self.path().parent().expect("has a parent");
        let metadata =
            async_fs::metadata(parent_path).await.with_context(|| {
                format!(
                    "couldn't get metadata for directory at {}",
                    parent_path
                )
            })?;
        Ok(Directory { path: parent_path.to_owned(), metadata })
    }

    #[inline]
    fn path(&self) -> &AbsPath {
        &self.path
    }

    #[inline]
    async fn read(&self) -> Result<Vec<u8>, Self::ReadError> {
        // We have to create a new file because reading needs an exclusive
        // borrow, and we don't have that here.
        //
        // TODO: can we read via &self if we don't move the cursor?
        let mut file = FileInner::open(self.path()).await?;
        let mut bytes = Vec::with_capacity(self.metadata.len() as usize);
        file.read_to_end(&mut bytes).await.with_context(|| {
            format!("couldn't read file at {}", self.path())
        })?;
        Ok(bytes)
    }

    #[inline]
    fn watch(&self) -> Self::EventStream {
        stream::pending()
    }

    #[inline]
    async fn write_chunks<Chunks, Chunk>(
        &mut self,
        chunks: Chunks,
    ) -> Result<(), Self::WriteError>
    where
        Chunks: IntoIterator<Item = Chunk> + Send,
        Chunks::IntoIter: Send,
        Chunk: AsRef<[u8]> + Send,
    {
        self.with_inner(async move |file, meta, path| {
            let write = async {
                for chunk in chunks {
                    file.write_all(chunk.as_ref()).await?;
                }
                file.sync_all().await
            };

            write.await.with_context(|| {
                format!("couldn't write to file at {path}")
            })?;

            let new_meta = file.metadata().await.with_context(|| {
                format!("couldn't get new metadata for file at {path}")
            })?;

            *meta = new_meta;

            Ok(())
        })
        .await?
    }
}

impl Deref for FileInner {
    type Target = async_fs::File;

    fn deref(&self) -> &async_fs::File {
        &self.inner
    }
}

impl DerefMut for FileInner {
    fn deref_mut(&mut self) -> &mut async_fs::File {
        &mut self.inner
    }
}
