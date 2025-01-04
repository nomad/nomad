use core::pin::Pin;
use core::task::{Context, Poll, ready};

use futures_executor::ThreadPool;
use nvimx_core::executor::{BackgroundExecutor, Task};

/// TODO: docs.
pub struct NeovimBackgroundExecutor {
    thread_pool: ThreadPool,
}

pin_project_lite::pin_project! {
    /// TODO: docs.
    pub struct NeovimBackgroundTask<T> {
        #[pin]
        inner: async_oneshot::Receiver<T>,
        is_forever_pending: bool,
    }
}

impl NeovimBackgroundExecutor {
    /// TODO: docs.
    #[inline]
    pub fn init() -> Self {
        Self {
            thread_pool: ThreadPool::builder()
                .name_prefix("nvimx")
                .create()
                .expect("couldn't create thread pool"),
        }
    }

    #[inline]
    fn spawn_inner<Fut>(
        &self,
        future: Fut,
    ) -> NeovimBackgroundTask<Fut::Output>
    where
        Fut: Future + Send + Sync + 'static,
        Fut::Output: Send + Sync + 'static,
    {
        let (mut tx, rx) = async_oneshot::oneshot();
        self.thread_pool.spawn_ok(async move {
            // The task might've been detached, and that's ok.
            let _ = tx.send(future.await);
        });
        NeovimBackgroundTask::new(rx)
    }
}

impl<T> NeovimBackgroundTask<T> {
    #[inline]
    pub(crate) fn new(inner: async_oneshot::Receiver<T>) -> Self {
        Self { inner, is_forever_pending: false }
    }
}

impl BackgroundExecutor for NeovimBackgroundExecutor {
    type Task<T> = NeovimBackgroundTask<T>;

    #[inline]
    fn spawn<Fut>(&mut self, future: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + Send + Sync + 'static,
        Fut::Output: Send + Sync + 'static,
    {
        self.spawn_inner(future)
    }
}

impl<T> Task<T> for NeovimBackgroundTask<T> {
    #[inline]
    fn detach(self) {}
}

impl<T> Future for NeovimBackgroundTask<T> {
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
