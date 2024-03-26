//! TODO: docs

use core::future::Future;
use core::pin::{pin, Pin};
use core::task::{Context, Poll};

use pin_project::pin_project;

/// TODO: docs
pub trait MaybeFuture: Sized {
    /// TODO: docs
    type Output;

    /// TODO: docs
    type Future: Future<Output = Self::Output>;

    /// TODO: docs
    fn into_enum(self) -> MaybeFutureEnum<Self::Future, Self::Output>;

    /// TODO: docs
    #[inline]
    fn into_future(self) -> impl Future<Output = Self::Output> {
        match self.into_enum() {
            MaybeFutureEnum::Ready(output) => {
                MaybeFutureFuture::Ready(core::future::ready(output))
            },

            MaybeFutureEnum::Future(future) => {
                MaybeFutureFuture::Future(future)
            },
        }
    }

    /// TODO: docs
    #[track_caller]
    #[inline]
    fn into_ready(self) -> Self::Output {
        match self.into_enum() {
            MaybeFutureEnum::Ready(output) => output,
            MaybeFutureEnum::Future(_) => panic!("future is not ready"),
        }
    }
}

/// TODO: docs
pub enum MaybeFutureEnum<F: Future<Output = T>, T> {
    /// TODO: docs
    Ready(T),

    /// TODO: docs
    Future(F),
}

impl<F: Future<Output = T>, T> MaybeFuture for MaybeFutureEnum<F, T> {
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

#[pin_project(project = MaybeFutureFutureProj)]
enum MaybeFutureFuture<F: Future<Output = T>, T> {
    Ready(core::future::Ready<T>),
    Future(#[pin] F),
}

impl<F: Future<Output = T>, T> Future for MaybeFutureFuture<F, T> {
    type Output = T;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            MaybeFutureFutureProj::Ready(future) => Pin::new(future).poll(cx),
            MaybeFutureFutureProj::Future(future) => future.poll(cx),
        }
    }
}

impls::ready!(());
impls::ready!(T; Option<T>);
impls::ready!(T, E; Result<T, E>);

mod impls {
    /// ..
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
