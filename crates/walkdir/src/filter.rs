use core::convert::Infallible;
use core::error::Error;

use ed::fs::{self, AbsPath, Metadata};
use futures_util::stream::{self, FusedStream, StreamExt};
use futures_util::{FutureExt, pin_mut, select};

use crate::WalkDir;

/// TODO: docs.
pub trait Filter<Fs: fs::Fs> {
    /// TODO: docs.
    type Error: Error + Send;

    /// TODO: docs.
    fn should_filter(
        &self,
        dir_path: &AbsPath,
        node_meta: &impl Metadata<Fs = Fs>,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send;

    /// TODO: docs.
    fn and<T>(self, other: T) -> And<Self, T>
    where
        T: Filter<Fs>,
        Self: Sized,
    {
        And { filter_1: self, filter_2: other }
    }
}

/// TODO: docs.
pub struct Filtered<F, W> {
    filter: F,
    walker: W,
}

/// TODO: docs.
pub struct And<F1, F2> {
    filter_1: F1,
    filter_2: F2,
}

/// TODO: docs.
#[derive(
    Debug, derive_more::Display, cauchy::Error, cauchy::PartialEq, cauchy::Eq,
)]
#[display("{_0}")]
pub enum Either<L, R> {
    /// TODO: docs.
    Left(L),
    /// TODO: docs.
    Right(R),
}

/// TODO: docs.
#[derive(
    cauchy::Debug,
    derive_more::Display,
    cauchy::Error,
    cauchy::PartialEq,
    cauchy::Eq,
)]
#[display("{_0}")]
pub enum FilteredEntryError<Fi, Fs, W>
where
    Fi: Filter<Fs>,
    Fs: fs::Fs,
    W: WalkDir<Fs>,
{
    /// TODO: docs.
    Filter(Fi::Error),

    /// TODO: docs.
    Walker(W::ReadEntryError),
}

impl<F, W> Filtered<F, W> {
    /// Consumes the `Filtered` and returns the underlying filter.
    #[inline]
    pub fn into_filter(self) -> F {
        self.filter
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn new(filter: F, walker: W) -> Self {
        Self { filter, walker }
    }
}

impl<Fs, Fi, W> WalkDir<Fs> for Filtered<Fi, W>
where
    Fs: fs::Fs,
    Fi: Sync + Filter<Fs>,
    W: Sync + WalkDir<Fs>,
{
    type ReadError = W::ReadError;
    type ReadEntryError = FilteredEntryError<Fi, Fs, W>;

    async fn read_dir(
        &self,
        dir_path: &AbsPath,
    ) -> Result<
        impl FusedStream<Item = Result<Fs::Metadata, Self::ReadEntryError>>,
        Self::ReadError,
    > {
        let entries = self.walker.read_dir(dir_path).await?;
        let filters = stream::FuturesUnordered::new();
        Ok(stream::unfold(
            (Box::pin(entries), filters),
            move |(mut entries, mut filters)| async move {
                let item = loop {
                    select! {
                        entry_res = entries.select_next_some() => {
                            let entry = match entry_res {
                                Ok(entry) => entry,
                                Err(err) => {
                                    break Err(FilteredEntryError::Walker(
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
                                break Err(FilteredEntryError::Filter(err));
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

impl<Fs: fs::Fs> Filter<Fs> for () {
    type Error = Infallible;

    async fn should_filter(
        &self,
        _: &AbsPath,
        _: &impl Metadata<Fs = Fs>,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }
}

impl<Fi, Fs> Filter<Fs> for &Fi
where
    Fi: Filter<Fs> + Sync,
    Fs: fs::Fs,
{
    type Error = Fi::Error;

    async fn should_filter(
        &self,
        dir_path: &AbsPath,
        node_meta: &impl Metadata<Fs = Fs>,
    ) -> Result<bool, Self::Error> {
        (*self).should_filter(dir_path, node_meta).await
    }
}

impl<Fi, Fs> Filter<Fs> for Option<Fi>
where
    Fi: Filter<Fs> + Sync,
    Fs: fs::Fs,
{
    type Error = Fi::Error;

    async fn should_filter(
        &self,
        dir_path: &AbsPath,
        node_meta: &impl Metadata<Fs = Fs>,
    ) -> Result<bool, Self::Error> {
        match self {
            Some(filter) => filter.should_filter(dir_path, node_meta).await,
            None => Ok(false),
        }
    }
}

impl<Fi1, Fi2, Fs> Filter<Fs> for And<Fi1, Fi2>
where
    Fi1: Filter<Fs> + Sync,
    Fi2: Filter<Fs> + Sync,
    Fs: fs::Fs,
{
    type Error = Either<Fi1::Error, Fi2::Error>;

    async fn should_filter(
        &self,
        dir_path: &AbsPath,
        node_meta: &impl Metadata<Fs = Fs>,
    ) -> Result<bool, Self::Error> {
        let filter_1 = self.filter_1.should_filter(dir_path, node_meta).fuse();
        let filter_2 = self.filter_2.should_filter(dir_path, node_meta).fuse();
        pin_mut!(filter_1);
        pin_mut!(filter_2);

        loop {
            if select! {
                res = filter_1 => res.map_err(Either::Left)?,
                res = filter_2 => res.map_err(Either::Right)?,
                complete => return Ok(false),
            } {
                return Ok(true);
            }
        }
    }
}

impl<Fi1, Fi2, Fs> Filter<Fs> for Either<Fi1, Fi2>
where
    Fi1: Filter<Fs> + Sync,
    Fi2: Filter<Fs> + Sync,
    Fs: fs::Fs,
{
    type Error = Either<Fi1::Error, Fi2::Error>;

    async fn should_filter(
        &self,
        dir_path: &AbsPath,
        node_meta: &impl Metadata<Fs = Fs>,
    ) -> Result<bool, Self::Error> {
        match self {
            Self::Left(filter) => filter
                .should_filter(dir_path, node_meta)
                .await
                .map_err(Either::Left),

            Self::Right(filter) => filter
                .should_filter(dir_path, node_meta)
                .await
                .map_err(Either::Right),
        }
    }
}
