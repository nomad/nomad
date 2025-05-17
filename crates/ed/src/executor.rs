//! TODO: docs.

use core::pin::Pin;
use core::task::{Context, Poll};

use crate::Backend;

/// TODO: docs.
pub trait Executor {
    /// TODO: docs.
    type Runner: Runner;

    /// TODO: docs.
    type LocalSpawner: LocalSpawner;

    /// TODO: docs.
    type BackgroundSpawner: BackgroundSpawner;

    /// TODO: docs.
    fn runner(&mut self) -> &mut Self::Runner;

    /// TODO: docs.
    fn local_spawner(&mut self) -> &mut Self::LocalSpawner;

    /// TODO: docs.
    fn background_spawner(&mut self) -> &mut Self::BackgroundSpawner;
}

/// TODO: docs.
pub trait Runner: Clone + 'static {
    /// TODO: docs.
    fn run<T>(
        &mut self,
        future: impl Future<Output = T>,
    ) -> impl Future<Output = T>;
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
    pub struct LocalTask<T, Ed: Backend> {
        #[pin]
        inner: <<<Ed as Backend>::Executor as Executor>::LocalSpawner as LocalSpawner>::Task<T>,
    }
}

pin_project_lite::pin_project! {
    /// TODO: docs.
    pub struct BackgroundTask<T, Ed: Backend> where T: 'static, T: Send {
        #[pin]
        inner: <<<Ed as Backend>::Executor as Executor>::BackgroundSpawner as BackgroundSpawner>::Task<T>,
    }
}

impl<T, Ed: Backend> LocalTask<T, Ed> {
    /// TODO: docs.
    #[inline]
    pub fn detach(self) {
        self.inner.detach();
    }

    #[inline]
    pub(crate) fn new(
        inner: <<<Ed as Backend>::Executor as Executor>::LocalSpawner as LocalSpawner>::Task<T>,
    ) -> Self {
        Self { inner }
    }
}

impl<T: Send + 'static, Ed: Backend> BackgroundTask<T, Ed> {
    /// TODO: docs.
    #[inline]
    pub fn detach(self) {
        self.inner.detach();
    }

    #[inline]
    pub(crate) fn new(
        inner: <<<Ed as Backend>::Executor as Executor>::BackgroundSpawner as BackgroundSpawner>::Task<T>,
    ) -> Self {
        Self { inner }
    }
}

impl<T, Ed: Backend> Future for LocalTask<T, Ed> {
    type Output = T;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        self.project().inner.poll(cx)
    }
}

impl<T: Send + 'static, Ed: Backend> Future for BackgroundTask<T, Ed> {
    type Output = T;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        self.project().inner.poll(cx)
    }
}
