use core::pin::Pin;
use core::task::{Context, Poll, ready};

use ed::executor::Task;

/// TODO: docs.
#[derive(Clone)]
pub struct ThreadPool {
    inner: futures_executor::ThreadPool,
}

pin_project_lite::pin_project! {
    /// TODO: docs.
    pub struct ThreadPoolTask<T> {
        #[pin]
        inner: async_oneshot::Receiver<T>,
        is_forever_pending: bool,
    }
}

impl ThreadPool {
    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: futures_executor::ThreadPool::builder()
                .create()
                .expect("couldn't create thread pool"),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn spawn<Fut>(&self, future: Fut) -> ThreadPoolTask<Fut::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let (mut tx, rx) = async_oneshot::oneshot();
        self.inner.spawn_ok(async move {
            // The task might've been detached, and that's ok.
            let _ = tx.send(future.await);
        });
        ThreadPoolTask::new(rx)
    }
}

impl<T> ThreadPoolTask<T> {
    #[inline]
    pub(crate) fn new(inner: async_oneshot::Receiver<T>) -> Self {
        Self { inner, is_forever_pending: false }
    }
}

impl Default for ThreadPool {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Task<T> for ThreadPoolTask<T> {
    #[inline]
    fn detach(self) {}
}

impl<T> Future for ThreadPoolTask<T> {
    type Output = T;

    #[inline]
    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<T> {
        let this = self.project();
        if *this.is_forever_pending {
            return Poll::Pending;
        }
        match ready!(this.inner.poll(ctx)) {
            Ok(value) => Poll::Ready(value),
            Err(_closed) => {
                // This only happens if the background executor is dropped,
                // which should only happen when Neovim is shutting down.
                *this.is_forever_pending = true;
                Poll::Pending
            },
        }
    }
}
