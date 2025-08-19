//! TODO: docs.

use core::convert::Infallible;
use core::error::Error;

use abs_path::AbsPath;
use either::Either;
use futures_util::{FutureExt, pin_mut, select_biased};

use crate::Metadata;

/// TODO: docs.
pub trait Filter<Fs: crate::Fs> {
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

/// A [`Filter`]-wrapper that filters entries when both of the inner filters
/// would filter the entry.
pub struct And<F1, F2> {
    filter_1: F1,
    filter_2: F2,
}

impl<Fs: crate::Fs> Filter<Fs> for () {
    type Error = Infallible;

    #[inline]
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
    Fs: crate::Fs,
{
    type Error = Fi::Error;

    #[inline]
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
    Fs: crate::Fs,
{
    type Error = Fi::Error;

    #[inline]
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
    Fs: crate::Fs,
{
    type Error = Either<Fi1::Error, Fi2::Error>;

    #[inline]
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
            if select_biased! {
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
    Fs: crate::Fs,
{
    type Error = Either<Fi1::Error, Fi2::Error>;

    #[inline]
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
