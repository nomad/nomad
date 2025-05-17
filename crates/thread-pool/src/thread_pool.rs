use core::pin::Pin;
use core::task::{Context, Poll, ready};

use ed::executor::{BackgroundSpawner, Task};

/// TODO: docs.
#[derive(Clone)]
pub struct ThreadPool {
    inner: futures_executor::ThreadPool,
}

pin_project_lite::pin_project! {
    /// TODO: docs.
    pub struct ThreadPoolTask<T: 'static> {
        #[pin]
        inner: flume::r#async::RecvFut<'static, T>,
        is_forever_pending: bool,
    }
}

impl ThreadPool {
    #[inline]
    fn spawn_inner<Fut>(&self, future: Fut) -> ThreadPoolTask<Fut::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let (tx, rx) = flume::bounded(1);
        self.inner.spawn_ok(async move {
            // The task might've been detached, and that's ok.
            let _ = tx.send(future.await);
        });
        ThreadPoolTask::new(rx.into_recv_async())
    }
}

impl<T> ThreadPoolTask<T> {
    #[inline]
    pub(crate) fn new(inner: flume::r#async::RecvFut<'static, T>) -> Self {
        Self { inner, is_forever_pending: false }
    }
}

impl Default for ThreadPool {
    #[track_caller]
    #[inline]
    fn default() -> Self {
        Self {
            inner: futures_executor::ThreadPool::builder()
                .create()
                .expect("couldn't create thread pool"),
        }
    }
}

impl BackgroundSpawner for ThreadPool {
    type Task<T: Send + 'static> = ThreadPoolTask<T>;

    #[inline]
    fn spawn<Fut>(&mut self, future: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        Self::spawn_inner(self, future)
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
            Err(flume::RecvError::Disconnected) => {
                // This only happens if the background executor is dropped,
                // which should only happen when Neovim is shutting down.
                *this.is_forever_pending = true;
                Poll::Pending
            },
        }
    }
}
