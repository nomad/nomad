//! TODO: docs

use core::future::Future;

/// TODO: docs
pub trait MaybeFuture {
    /// TODO: docs
    type Output;

    /// TODO: docs
    type Future;

    /// TODO: docs
    fn into_enum(self) -> MaybeFutureEnum<Self::Future, Self::Output>;
}

/// TODO: docs
pub enum MaybeFutureEnum<F, T> {
    /// TODO: docs
    Ready(T),

    /// TODO: docs
    Future(F),
}

impl<F, T> MaybeFuture for MaybeFutureEnum<F, T> {
    type Future = F;

    type Output = T;

    #[inline]
    fn into_enum(self) -> MaybeFutureEnum<F, T> {
        self
    }
}

impl<F, T> From<F> for MaybeFutureEnum<F, T>
where
    F: Future<Output = T>,
{
    #[inline]
    fn from(future: F) -> Self {
        MaybeFutureEnum::Future(future)
    }
}

impls::ready!(());
impls::ready!(T; Option<T>);
impls::ready!(T, E; Result<T, E>);

mod impls {
    /// ..
    #[macro_export]
    macro_rules! ready {
        ($ty:ty) => {
            impl MaybeFuture for $ty {
                type Output = Self;

                type Future = ::core::future::Ready<Self>;

                #[inline]
                fn into_enum(self) -> MaybeFutureEnum<Self::Future, Self> {
                    MaybeFutureEnum::Ready(self)
                }
            }
        };

        ($($gen:ident),*; $ty:ty) => {
            impl<$($gen),*> MaybeFuture for $ty {
                type Output = Self;

                type Future = ::core::future::Ready<Self>;

                #[inline]
                fn into_enum(self) -> MaybeFutureEnum<Self::Future, Self> {
                    MaybeFutureEnum::Ready(self)
                }
            }
        };
    }

    pub(super) use ready;
}
