use core::error::Error;
use core::pin::Pin;

use abs_path::{AbsPath, AbsPathBuf};
use ed::fs::{self, Directory, Metadata};
use futures_util::stream::{self, FusedStream, StreamExt};
use futures_util::{FutureExt, select_biased};

use crate::filter::{Filter, Filtered};

/// TODO: docs.
pub trait WalkDir<Fs: fs::Fs>: Sized {
    /// The type of error that can occur when reading a directory fails.
    type ListError: Error + Send;

    /// The type of error that can occur when reading the metadata of a node in
    /// a directory fails.
    type ReadMetadataError: Error + Send;

    /// TODO: docs.
    fn list_metas(
        &self,
        dir_path: &AbsPath,
    ) -> impl Future<
        Output = Result<
            impl FusedStream<Item = Result<Fs::Metadata, Self::ReadMetadataError>>
            + Send
            + Unpin,
            Self::ListError,
        >,
    > + Send;

    /// TODO: docs.
    #[inline]
    fn filter<F>(self, filter: F) -> Filtered<F, Self>
    where
        F: Filter<Fs>,
    {
        Filtered::new(filter, self)
    }

    /// TODO: docs.
    #[inline]
    fn for_each<Err: Send>(
        &self,
        dir_path: &AbsPath,
        handler: impl ForEachHandler<Result<(), Err>, Fs>,
    ) -> impl Future<Output = Result<(), WalkError<Fs, Self, Err>>> + Send
    where
        Self: Sync,
    {
        #[allow(clippy::type_complexity)]
        #[inline]
        fn inner<'a, W, Err, Fs>(
            walkdir: &'a W,
            dir_path: &'a AbsPath,
            handler: impl ForEachHandler<Result<(), Err>, Fs> + 'a,
        ) -> Pin<
            Box<
                dyn Future<Output = Result<(), WalkError<Fs, W, Err>>>
                    + Send
                    + 'a,
            >,
        >
        where
            W: WalkDir<Fs> + Sync,
            Err: Send + 'a,
            Fs: fs::Fs,
        {
            Box::pin(async move {
                let mut entries = walkdir
                    .list_metas(dir_path)
                    .await
                    .map_err(WalkError::ListDir)?;
                let mut handle_entries = stream::FuturesUnordered::new();
                let mut read_children = stream::FuturesUnordered::new();
                loop {
                    select_biased! {
                        res = entries.select_next_some() => {
                            let meta = res.map_err(WalkError::ReadMetadata)?;
                            let node_kind = meta.node_kind();
                            if node_kind.is_dir() {
                                let dir_name = meta
                                    .name()
                                    .map_err(WalkError::NodeName)?;
                                let dir_path = dir_path.join(dir_name);
                                let handler = handler.clone();
                                read_children.push(async move {
                                    inner(walkdir, &dir_path, handler).await
                                });
                            }
                            let handler = handler.clone();
                            handle_entries.push(async move {
                                handler.async_call_once((dir_path, meta)).await
                            });
                        },
                        res = read_children.select_next_some() => res?,
                        res = handle_entries.select_next_some() => {
                            res.map_err(WalkError::Other)?;
                        },
                        complete => return Ok(()),
                    }
                }
            })
        }

        async move { inner(self, dir_path, handler).await }
    }

    /// TODO: docs.
    #[inline]
    fn paths<'a>(
        &'a self,
        dir_path: &'a AbsPath,
    ) -> impl FusedStream<
        Item = Result<AbsPathBuf, WalkError<Fs, Self, fs::MetadataNameError>>,
    > + Send
    + 'a
    where
        Self: Sync,
    {
        self.to_stream(dir_path, async move |dir_path, meta| {
            meta.name().map(|name| dir_path.join(name))
        })
    }

    /// TODO: docs.
    #[inline]
    fn to_stream<'a, T: Send + 'a, E: Send + 'a>(
        &'a self,
        dir_path: &'a AbsPath,
        handler: impl ForEachHandler<Result<T, E>, Fs> + 'a,
    ) -> impl FusedStream<Item = Result<T, WalkError<Fs, Self, E>>> + Send + 'a
    where
        Self: Sync,
    {
        let (tx, rx) = flume::unbounded();
        let for_each = self
            .for_each(dir_path, async move |dir_path, metadata| {
                let _ = tx.send(handler(dir_path, metadata).await?);
                Ok(())
            })
            .boxed()
            .fuse();
        futures_util::stream::unfold(
            (for_each, rx.into_stream()),
            move |(mut for_each, mut rx_stream)| async move {
                let res = loop {
                    select_biased! {
                        res = &mut for_each => match res {
                            Ok(()) => {},
                            Err(err) => break Err(err),
                        },
                        value = rx_stream.select_next_some() => {
                            break Ok(value)
                        },
                        complete => return None,
                    }
                };
                Some((res, (for_each, rx_stream)))
            },
        )
    }
}

/// TODO: docs.
#[derive(
    cauchy::Debug, derive_more::Display, cauchy::Error, cauchy::PartialEq,
)]
#[display("{_0}")]
pub enum WalkError<Fs, W, T>
where
    Fs: fs::Fs,
    W: WalkDir<Fs>,
{
    /// TODO: docs.
    ListDir(W::ListError),

    /// TODO: docs.
    Other(T),

    /// TODO: docs.
    NodeName(fs::MetadataNameError),

    /// TODO: docs.
    ReadMetadata(W::ReadMetadataError),
}

/// TODO: docs.
#[derive(
    cauchy::Debug, derive_more::Display, cauchy::Error, cauchy::PartialEq,
)]
#[display("{_0}")]
pub enum FsReadDirError<Fs: fs::Fs> {
    /// TODO: docs.
    ListDir(<Fs::Directory as fs::Directory>::ListError),

    /// TODO: docs.
    #[display("no node at path")]
    NoNodeAtPath,

    /// TODO: docs.
    NodeAtPath(Fs::NodeAtPathError),

    /// TODO: docs.
    #[display("couldn't read file at path")]
    ReadFile,

    /// TODO: docs.
    #[display("couldn't read symlink at path")]
    ReadSymlink,
}

impl<Fs: fs::Fs> WalkDir<Self> for Fs {
    type ListError = FsReadDirError<Self>;
    type ReadMetadataError =
        <Fs::Directory as fs::Directory>::ReadMetadataError;

    async fn list_metas(
        &self,
        dir_path: &AbsPath,
    ) -> Result<
        impl FusedStream<
            Item = Result<<Self as fs::Fs>::Metadata, Self::ReadMetadataError>,
        > + Send,
        Self::ListError,
    > {
        let Some(node) = self
            .node_at_path(dir_path)
            .await
            .map_err(FsReadDirError::NodeAtPath)?
        else {
            return Err(FsReadDirError::NoNodeAtPath);
        };

        match node {
            fs::FsNode::Directory(dir) => dir
                .list_metas()
                .await
                .map(StreamExt::fuse)
                .map_err(FsReadDirError::ListDir),
            fs::FsNode::File(_) => Err(FsReadDirError::ReadFile),
            fs::FsNode::Symlink(_) => Err(FsReadDirError::ReadSymlink),
        }
    }
}

pub trait ForEachHandler<Out, Fs: fs::Fs>:
    for<'a> AsyncFnOnce<
        (&'a AbsPath, Fs::Metadata),
        CallOnceFuture: Send,
        Output = Out,
    > + Send
    + Clone
{
}

impl<H, Out, Fs> ForEachHandler<Out, Fs> for H
where
    H: for<'a> AsyncFnOnce<
            (&'a AbsPath, Fs::Metadata),
            CallOnceFuture: Send,
            Output = Out,
        > + Send
        + Clone,
    Fs: fs::Fs,
{
}
