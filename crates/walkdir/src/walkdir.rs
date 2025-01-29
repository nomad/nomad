use core::convert::Infallible;
use core::pin::Pin;

use futures_util::stream::{self, Stream, StreamExt};
use futures_util::{FutureExt, pin_mut, select};
use nvimx2::fs;

use crate::dir_entry::DirEntry;
use crate::filter::{Either, Filter, Filtered};

/// TODO: docs.
pub trait WalkDir: Sized {
    /// TODO: docs.
    type DirEntry: fs::DirEntry;

    /// TODO: docs.
    type DirEntryError;

    /// TODO: docs.
    type ReadDirError;

    /// TODO: docs.
    fn read_dir(
        &self,
        dir_path: &fs::AbsPath,
    ) -> impl Future<
        Output = Result<
            impl Stream<Item = Result<Self::DirEntry, Self::DirEntryError>>,
            Self::ReadDirError,
        >,
    >;

    /// TODO: docs.
    #[inline]
    fn filter<F>(self, filter: F) -> Filtered<F, Self>
    where
        F: Filter<Self>,
    {
        Filtered::new(filter, self)
    }

    /// TODO: docs.
    #[allow(clippy::type_complexity)]
    #[inline]
    fn for_each<'a, H, E>(
        &'a self,
        dir_path: &'a fs::AbsPath,
        handler: H,
    ) -> Pin<Box<dyn Future<Output = Result<(), ForEachError<Self, E>>> + 'a>>
    where
        H: AsyncFn(DirEntry<Self>) -> Result<(), E> + Clone + 'a,
        E: 'a,
    {
        Box::pin(async move {
            let entries = match self.read_dir(dir_path).await {
                Ok(entries) => entries.fuse(),
                Err(err) => {
                    return Err(ForEachError {
                        dir_path: dir_path.to_owned(),
                        kind: Either::Left(WalkErrorKind::ReadDir(err)),
                    });
                },
            };
            let mut create_entries = stream::FuturesUnordered::new();
            let mut handle_entries = stream::FuturesUnordered::new();
            let mut read_children = stream::FuturesUnordered::new();
            pin_mut!(entries);
            loop {
                select! {
                    res = entries.select_next_some() => {
                        let entry = res.map_err(|err| ForEachError {
                            dir_path: dir_path.to_owned(),
                            kind: Either::Left(WalkErrorKind::DirEntry(err)),
                        })?;
                        create_entries.push(DirEntry::new(dir_path, entry));
                    },
                    res = create_entries.select_next_some() => {
                        let entry = res.map_err(|kind| ForEachError {
                            dir_path: dir_path.to_owned(),
                            kind: Either::Left(kind),
                        })?;
                        if entry.node_kind().is_dir() {
                            let dir_path = entry.path();
                            let handler = handler.clone();
                            read_children.push(async move {
                                self.for_each(&dir_path, handler).await
                            });
                        }
                        let handler = &handler;
                        handle_entries.push(async move {
                            let parent_path = entry.parent_path();
                            handler(entry).await.map_err(|err| {
                                ForEachError {
                                    dir_path: parent_path.to_owned(),
                                    kind: Either::Right(err),
                                }
                            })
                        });
                    },
                    res = read_children.select_next_some() => res?,
                    res = handle_entries.select_next_some() => res?,
                    complete => return Ok(()),
                }
            }
        })
    }

    /// TODO: docs.
    #[inline]
    fn paths<'a>(
        &'a self,
        dir_path: &'a fs::AbsPath,
    ) -> impl Stream<Item = Result<fs::AbsPathBuf, PathsError<Self>>> + 'a
    {
        self.to_stream(dir_path, async |entry| {
            Ok::<_, Infallible>(entry.path())
        })
        .map(|res| {
            res.map_err(|err| {
                err.map_kind(|kind| match kind {
                    Either::Left(res) => res,
                    Either::Right(_infallible) => unreachable!(),
                })
            })
        })
    }

    /// TODO: docs.
    #[inline]
    fn to_stream<'a, H, T, E>(
        &'a self,
        dir_path: &'a fs::AbsPath,
        handler: H,
    ) -> impl Stream<Item = Result<T, ForEachError<Self, E>>> + 'a
    where
        H: AsyncFn(DirEntry<Self>) -> Result<T, E> + Clone + 'a,
        T: 'a,
        E: 'a,
    {
        let (tx, rx) = flume::unbounded();
        let for_each = self
            .for_each(dir_path, async move |entry| {
                let _ = tx.send(handler(entry).await?);
                Ok(())
            })
            .boxed_local()
            .fuse();
        futures_util::stream::unfold(
            (for_each, rx),
            move |(mut for_each, rx)| async move {
                let res = select! {
                    res = for_each => match res {
                        Ok(()) => return None,
                        Err(err) => Err(err),
                    },
                    res = rx.recv_async() => match res {
                        Ok(value) => Ok(value),
                        Err(_err) => return None,
                    },
                };
                Some((res, (for_each, rx)))
            },
        )
    }
}

/// TODO: docs.
pub type ForEachError<W, E> = WalkError<Either<WalkErrorKind<W>, E>>;

/// TODO: docs.
pub type PathsError<W> = WalkError<WalkErrorKind<W>>;

/// TODO: docs.
pub struct WalkError<K> {
    /// TODO: docs.
    pub dir_path: fs::AbsPathBuf,

    /// TODO: docs.
    pub kind: K,
}

/// TODO: docs.
pub enum WalkErrorKind<W: WalkDir> {
    /// TODO: docs.
    DirEntry(W::DirEntryError),

    /// TODO: docs.
    DirEntryName(<W::DirEntry as fs::DirEntry>::NameError),

    /// TODO: docs.
    DirEntryNodeKind(<W::DirEntry as fs::DirEntry>::NodeKindError),

    /// TODO: docs.
    ReadDir(W::ReadDirError),
}

impl<K> WalkError<K> {
    /// TODO: docs.
    pub fn map_kind<F, K2>(self, f: F) -> WalkError<K2>
    where
        F: FnOnce(K) -> K2,
    {
        WalkError { dir_path: self.dir_path, kind: f(self.kind) }
    }
}

impl<Fs: fs::Fs> WalkDir for Fs {
    type DirEntry = <Self as fs::Fs>::DirEntry;
    type DirEntryError = <Self as fs::Fs>::DirEntryError;
    type ReadDirError = <Self as fs::Fs>::ReadDirError;

    async fn read_dir(
        &self,
        dir_path: &fs::AbsPath,
    ) -> Result<<Self as fs::Fs>::ReadDir, Self::ReadDirError> {
        fs::Fs::read_dir(self, dir_path).await
    }
}
