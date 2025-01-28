use futures_util::select;
use futures_util::stream::{self, FuturesUnordered, Stream, StreamExt};
use nvimx2::fs;

use crate::WalkDir;

/// TODO: docs.
pub trait Filter<W: WalkDir> {
    /// TODO: docs.
    type Error;

    /// TODO: docs.
    fn should_filter(
        &self,
        dir_path: &fs::AbsPath,
        dir_entry: &W::DirEntry,
    ) -> impl Future<Output = Result<bool, Self::Error>>;
}

/// TODO: docs.
pub struct Filtered<F, W> {
    filter: F,
    walker: W,
}

/// TODO: docs.
pub enum FilteredDirEntryError<F, W>
where
    F: Filter<W>,
    W: WalkDir,
{
    /// TODO: docs.
    Filter(F::Error),

    /// TODO: docs.
    Walker(W::DirEntryError),
}

impl<F, W> Filtered<F, W> {
    /// TODO: docs.
    #[inline]
    pub(crate) fn new(filter: F, walker: W) -> Self {
        Self { filter, walker }
    }
}

impl<F, W> WalkDir for Filtered<F, W>
where
    F: Filter<W>,
    W: WalkDir,
{
    type DirEntry = W::DirEntry;
    type DirEntryError = FilteredDirEntryError<F, W>;
    type ReadDirError = W::ReadDirError;

    async fn read_dir(
        &self,
        dir_path: &fs::AbsPath,
    ) -> Result<
        impl Stream<Item = Result<Self::DirEntry, Self::DirEntryError>>,
        Self::ReadDirError,
    > {
        let entries = self.walker.read_dir(dir_path).await?.fuse();
        let filters = FuturesUnordered::new();
        Ok(stream::unfold(
            (Box::pin(entries), filters),
            move |(mut entries, mut filters)| async move {
                let item = loop {
                    select! {
                        entry_res = entries.select_next_some() => {
                            let entry = match entry_res {
                                Ok(entry) => entry,
                                Err(err) => {
                                    break Err(FilteredDirEntryError::Walker(
                                        err,
                                    ));
                                },
                            };
                            filters.push(async move {
                                self.filter
                                    .should_filter(dir_path, &entry)
                                    .await
                                    .map(|filtr| (entry, filtr))
                            });
                        },
                        res = filters.select_next_some() => match res {
                            Ok((entry, false)) => break Ok(entry),
                            Err(err) => {
                                break Err(FilteredDirEntryError::Filter(err));
                            },
                            Ok((_, true)) => (),
                        },
                        complete => return None,
                    }
                };
                Some((item, (entries, filters)))
            },
        ))
    }
}

impl<F, E, W> Filter<W> for F
where
    F: AsyncFn(&fs::AbsPath, &W::DirEntry) -> Result<bool, E>,
    W: WalkDir,
{
    type Error = E;

    async fn should_filter(
        &self,
        dir_path: &fs::AbsPath,
        dir_entry: &<W as WalkDir>::DirEntry,
    ) -> Result<bool, Self::Error> {
        self(dir_path, dir_entry).await
    }
}
