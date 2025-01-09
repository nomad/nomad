//! TODO: docs.

use core::pin::Pin;
use core::task::{Context, Poll};

use crate::backend::Backend;

/// TODO: docs.
pub trait LocalExecutor {
    /// TODO: docs.
    type Task<T>: Task<T>;

    /// TODO: docs.
    fn spawn<Fut>(&mut self, f: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static;
}

/// TODO: docs.
pub trait BackgroundExecutor {
    /// TODO: docs.
    type Task<T>: Task<T>;

    /// TODO: docs.
    fn spawn<Fut>(&mut self, f: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static;
}

/// TODO: docs.
pub trait Task<T>: Future<Output = T> {
    /// TODO: docs.
    fn detach(self);
}

pin_project_lite::pin_project! {
    /// TODO: docs.
    pub struct TaskBackground<T, B: Backend> {
        #[pin]
        inner: <<B as Backend>::BackgroundExecutor as BackgroundExecutor>::Task<T>,
    }
}

impl<T, B: Backend> TaskBackground<T, B> {
    /// TODO: docs.
    #[inline]
    pub fn detach(self) {
        self.inner.detach();
    }

    #[inline]
    pub(crate) fn new(
        inner: <<B as Backend>::BackgroundExecutor as BackgroundExecutor>::Task<T>,
    ) -> Self {
        Self { inner }
    }
}

impl<T, B: Backend> Future for TaskBackground<T, B> {
    type Output = T;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().inner.poll(cx)
    }
}
