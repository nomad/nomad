use futures_util::{FutureExt, Stream, StreamExt, select};
use nvimx2::fs::{self, DirEntry};

use crate::accumulate::{self, AccumulateError, Accumulator};
use crate::filter::{Filter, Filtered};

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
    fn accumulate<A, Fs>(
        &self,
        acc: &mut A,
        fs: &mut Fs,
    ) -> impl Future<Output = Result<Fs::Timestamp, AccumulateError<A, Self, Fs>>>
    where
        A: Accumulator<Fs>,
        Fs: fs::Fs,
    {
        async move { accumulate::accumulate(self, acc, fs).await }
    }

    /// TODO: docs.
    #[inline]
    fn filter<F>(self, filter: F) -> Filtered<F, Self>
    where
        F: Filter<Self>,
    {
        Filtered::new(filter, self)
    }

    /// TODO: docs.
    #[inline]
    fn for_each<H: AsyncFn(&fs::AbsPath, Self::DirEntry)>(
        &self,
        _dir_path: fs::AbsPathBuf,
        _handler: H,
    ) -> impl Future<Output = Result<(), WalkError<Self>>> {
        async move {
            todo!();
        }
    }

    /// TODO: docs.
    #[inline]
    fn paths(
        &self,
        dir_path: fs::AbsPathBuf,
    ) -> impl Stream<Item = Result<fs::AbsPathBuf, PathsError<Self>>> {
        self.to_stream(dir_path, async |parent_path, entry| {
            let mut path = parent_path.to_owned();
            let entry_name = entry.name().await?;
            path.push(entry_name);
            Ok(path)
        })
        .map(|res| match res {
            Ok(Ok(path)) => Ok(path),
            Ok(Err(err)) => Err(PathsError::DirEntryName(err)),
            Err(err) => Err(PathsError::Walk(err)),
        })
    }

    /// TODO: docs.
    #[inline]
    fn to_stream<'a, H, T>(
        &'a self,
        dir_path: fs::AbsPathBuf,
        handler: H,
    ) -> impl Stream<Item = Result<T, WalkError<Self>>> + 'a
    where
        H: AsyncFn(&fs::AbsPath, Self::DirEntry) -> T + 'a,
        T: 'a,
    {
        let (tx, rx) = flume::unbounded();
        let for_each = self
            .for_each(dir_path, async move |path, entry| {
                let payload = handler(path, entry).await;
                let _ = tx.send(payload);
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
                        Ok(payload) => Ok(payload),
                        Err(_err) => return None,
                    },
                };
                Some((res, (for_each, rx)))
            },
        )
    }
}

/// TODO: docs.
pub enum PathsError<W: WalkDir> {
    /// TODO: docs.
    Walk(WalkError<W>),

    /// TODO: docs.
    DirEntryName(<W::DirEntry as fs::DirEntry>::NameError),
}

/// TODO: docs.
pub struct WalkError<W: WalkDir> {
    /// TODO: docs.
    pub dir_path: fs::AbsPathBuf,

    /// TODO: docs.
    pub kind: WalkErrorKind<W>,
}

/// TODO: docs.
pub enum WalkErrorKind<W: WalkDir> {
    /// TODO: docs.
    DirEntry(W::DirEntryError),

    /// TODO: docs.
    ReadDir(W::ReadDirError),
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
