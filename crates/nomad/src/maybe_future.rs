//! TODO: docs

use core::future::Future;
use core::pin::Pin;

/// TODO: docs
pub trait MaybeFuture<'a> {
    /// TODO: docs
    type Output;

    /// TODO: docs
    fn into_enum(self) -> MaybeFutureEnum<'a, Self::Output>;
}

impl MaybeFuture<'static> for () {
    type Output = ();

    #[inline]
    fn into_enum(self) -> MaybeFutureEnum<'static, ()> {
        MaybeFutureEnum::Ready(())
    }
}

/// TODO: docs
pub enum MaybeFutureEnum<'a, T> {
    /// TODO: docs
    Ready(T),

    /// TODO: docs
    Future(Pin<Box<dyn Future<Output = T> + 'a>>),
}

impl<'a, T> MaybeFuture<'a> for MaybeFutureEnum<'a, T> {
    type Output = T;

    #[inline]
    fn into_enum(self) -> MaybeFutureEnum<'a, T> {
        self
    }
}

impl<'a, F, T> From<F> for MaybeFutureEnum<'a, T>
where
    F: Future<Output = T> + 'a,
{
    #[inline]
    fn from(future: F) -> Self {
        MaybeFutureEnum::Future(Box::pin(future))
    }
}
