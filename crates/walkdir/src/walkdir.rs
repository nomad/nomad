use core::error::Error;
use core::pin::Pin;

use abs_path::{AbsPath, AbsPathBuf};
use ed::fs::{self, Directory, Metadata};
use futures_util::stream::{self, FusedStream, StreamExt};
use futures_util::{FutureExt, pin_mut, select};

use crate::filter::{Filter, Filtered};

/// TODO: docs.
pub trait WalkDir<Fs: fs::Fs>: Sized {
    /// The type of error that can occur when reading a directory fails.
    type ReadError: Error + Send;

    /// The type of error that can occur when reading a specific entry in a
    /// directory fails.
    type ReadEntryError: Error + Send;

    /// TODO: docs.
    fn read_dir(
        &self,
        dir_path: &AbsPath,
    ) -> impl Future<
        Output = Result<
            impl FusedStream<Item = Result<Fs::Metadata, Self::ReadEntryError>>
            + Send,
            Self::ReadError,
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
    fn for_each<'a, Err: Send + 'a>(
        &'a self,
        dir_path: &'a AbsPath,
        handler: impl ForEachHandler<Fs, Err> + 'a,
    ) -> impl Future<Output = Result<(), WalkError<Fs, Self, Err>>> + Send
    where
        Self: Sync,
    {
        #[allow(clippy::type_complexity)]
        #[inline]
        fn inner<'a, W, Err, Fs>(
            walkdir: &'a W,
            dir_path: &'a AbsPath,
            handler: impl ForEachHandler<Fs, Err> + 'a,
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
                let entries = walkdir
                    .read_dir(dir_path)
                    .await
                    .map_err(WalkError::ReadDir)?;
                let mut handle_entries = stream::FuturesUnordered::new();
                let mut read_children = stream::FuturesUnordered::new();
                pin_mut!(entries);
                loop {
                    select! {
                        res = entries.select_next_some() => {
                            let entry = res.map_err(WalkError::ReadEntry)?;
                            let node_kind = entry.node_kind();
                            if node_kind.is_dir() {
                                let dir_name = entry
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
                                handler.async_call_once((dir_path, entry)).await
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

    // /// TODO: docs.
    // #[inline]
    // fn paths<'a>(
    //     &'a self,
    //     dir_path: &'a AbsPath,
    // ) -> impl FusedStream<
    //     Item = Result<AbsPathBuf, WalkError<Fs, Self, fs::MetadataNameError>>,
    // > + 'a
    // where
    //     Self: Sync,
    // {
    //     self.to_stream(dir_path, async |dir_path, entry| {
    //         entry.name().map(|name| dir_path.join(name))
    //     })
    // }
    //
    // /// TODO: docs.
    // #[inline]
    // fn to_stream<'a, T, E>(
    //     &'a self,
    //     dir_path: &'a AbsPath,
    //     handler: impl AsyncFnOnce(&AbsPath, Fs::Metadata) -> Result<T, E>
    //     + Send
    //     + Clone
    //     + 'a,
    // ) -> impl FusedStream<Item = Result<T, WalkError<Fs, Self, E>>> + 'a
    // where
    //     Self: Sync,
    //     T: Send + 'a,
    //     E: Send + 'a,
    // {
    //     let (tx, rx) = flume::unbounded();
    //     let for_each = self
    //         .for_each(dir_path, async move |dir_path, entry| {
    //             let _ = tx.send(handler(dir_path, entry).await?);
    //             Ok(())
    //         })
    //         .boxed_local()
    //         .fuse();
    //     futures_util::stream::unfold(
    //         (for_each, rx),
    //         move |(mut for_each, rx)| async move {
    //             let res = select! {
    //                 res = for_each => match res {
    //                     Ok(()) => return None,
    //                     Err(err) => Err(err),
    //                 },
    //                 res = rx.recv_async() => match res {
    //                     Ok(value) => Ok(value),
    //                     Err(_err) => return None,
    //                 },
    //             };
    //             Some((res, (for_each, rx)))
    //         },
    //     )
    // }
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
    Other(T),

    /// TODO: docs.
    NodeName(fs::MetadataNameError),

    /// TODO: docs.
    ReadDir(W::ReadError),

    /// TODO: docs.
    ReadEntry(W::ReadEntryError),
}

/// TODO: docs.
#[derive(
    cauchy::Debug, derive_more::Display, cauchy::Error, cauchy::PartialEq,
)]
#[display("{_0}")]
pub enum FsReadDirError<Fs: fs::Fs> {
    /// TODO: docs.
    #[display("no node at path")]
    NoNodeAtPath,

    /// TODO: docs.
    NodeAtPath(Fs::NodeAtPathError),

    /// TODO: docs.
    ReadDir(<Fs::Directory as fs::Directory>::ReadError),

    /// TODO: docs.
    #[display("couldn't read file at path")]
    ReadFile,

    /// TODO: docs.
    #[display("couldn't read symlink at path")]
    ReadSymlink,
}

impl<Fs: fs::Fs> WalkDir<Self> for Fs {
    type ReadError = FsReadDirError<Self>;
    type ReadEntryError = <Fs::Directory as fs::Directory>::ReadEntryError;

    async fn read_dir(
        &self,
        dir_path: &fs::AbsPath,
    ) -> Result<
        impl FusedStream<
            Item = Result<<Self as fs::Fs>::Metadata, Self::ReadEntryError>,
        > + Send,
        Self::ReadError,
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
                .read()
                .await
                .map(StreamExt::fuse)
                .map_err(FsReadDirError::ReadDir),
            fs::FsNode::File(_) => Err(FsReadDirError::ReadFile),
            fs::FsNode::Symlink(_) => Err(FsReadDirError::ReadSymlink),
        }
    }
}

pub trait ForEachHandler<Fs: fs::Fs, Err>:
    for<'a> AsyncFnOnce<
        (&'a AbsPath, Fs::Metadata),
        CallOnceFuture: Send,
        Output = Result<(), Err>,
    > + Send
    + Clone
{
}

impl<Fs, Err, H> ForEachHandler<Fs, Err> for H
where
    Fs: fs::Fs,
    H: for<'a> AsyncFnOnce<
            (&'a AbsPath, Fs::Metadata),
            CallOnceFuture: Send,
            Output = Result<(), Err>,
        > + Send
        + Clone,
{
}
