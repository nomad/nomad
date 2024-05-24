use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project_lite::pin_project;

pin_project! {
    /// TODO: docs
    pub struct Task<T> {
        #[pin]
        inner: async_task::Task<T>,
    }
}

impl<T> Task<T> {
    /// TODO: docs
    #[inline]
    pub fn detach(self) {
        self.inner.detach()
    }

    /// Creates a new [`Task`].
    #[inline]
    pub(crate) fn new(inner: async_task::Task<T>) -> Self {
        Self { inner }
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    #[inline]
    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<T> {
        self.project().inner.poll(ctx)
    }
}
