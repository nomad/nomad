//! TODO: docs.

use core::pin::Pin;
use core::task::{Context, Poll};

use crate::Editor;

/// TODO: docs.
pub trait Executor {
    /// TODO: docs.
    type LocalSpawner: LocalSpawner;

    /// TODO: docs.
    type BackgroundSpawner: BackgroundSpawner;

    /// TODO: docs.
    fn run<Fut: Future>(
        &mut self,
        future: Fut,
    ) -> impl Future<Output = Fut::Output> + use<Self, Fut>;

    /// TODO: docs.
    fn local_spawner(&mut self) -> &mut Self::LocalSpawner;

    /// TODO: docs.
    fn background_spawner(&mut self) -> &mut Self::BackgroundSpawner;

    /// Blocks the current thread until the given future completes.
    #[inline]
    fn block_on<T>(&mut self, future: impl Future<Output = T>) -> T {
        futures_lite::future::block_on(self.run(future))
    }
}

/// TODO: docs.
pub trait LocalSpawner {
    /// TODO: docs.
    type Task<T>: Task<T>;

    /// TODO: docs.
    fn spawn<Fut>(&mut self, fut: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static;
}

/// TODO: docs.
pub trait BackgroundSpawner: Clone + Send + 'static {
    /// TODO: docs.
    type Task<T: Send + 'static>: Task<T> + Send;

    /// TODO: docs.
    fn spawn<Fut>(&mut self, fut: Fut) -> Self::Task<Fut::Output>
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
    pub struct LocalTask<T, Ed: Editor> {
        #[pin]
        inner: <<<Ed as Editor>::Executor as Executor>::LocalSpawner as LocalSpawner>::Task<T>,
    }
}

pin_project_lite::pin_project! {
    /// TODO: docs.
    pub struct BackgroundTask<T, Ed: Editor> where T: 'static, T: Send {
        #[pin]
        inner: <<<Ed as Editor>::Executor as Executor>::BackgroundSpawner as BackgroundSpawner>::Task<T>,
    }
}

impl<T, Ed: Editor> LocalTask<T, Ed> {
    /// TODO: docs.
    #[inline]
    pub fn detach(self) {
        self.inner.detach();
    }

    #[inline]
    pub(crate) fn new(
        inner: <<<Ed as Editor>::Executor as Executor>::LocalSpawner as LocalSpawner>::Task<T>,
    ) -> Self {
        Self { inner }
    }
}

impl<T: Send + 'static, Ed: Editor> BackgroundTask<T, Ed> {
    /// TODO: docs.
    #[inline]
    pub fn detach(self) {
        self.inner.detach();
    }

    #[inline]
    pub(crate) fn new(
        inner: <<<Ed as Editor>::Executor as Executor>::BackgroundSpawner as BackgroundSpawner>::Task<T>,
    ) -> Self {
        Self { inner }
    }
}

impl<T, Ed: Editor> Future for LocalTask<T, Ed> {
    type Output = T;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        self.project().inner.poll(cx)
    }
}

impl<T: Send + 'static, Ed: Editor> Future for BackgroundTask<T, Ed> {
    type Output = T;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        self.project().inner.poll(cx)
    }
}

#[cfg(feature = "async-task")]
impl<T> Task<T> for async_task::Task<T> {
    #[inline]
    fn detach(self) {
        Self::detach(self);
    }
}
