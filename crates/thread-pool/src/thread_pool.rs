use core::pin::Pin;
use core::task::{Context, Poll, ready};
use std::panic;

use executor::{BackgroundSpawner, Task};
use futures_lite::FutureExt;

type PanicPayload = Box<dyn core::any::Any + Send + 'static>;

/// TODO: docs.
#[derive(Clone)]
pub struct ThreadPool {
    inner: futures_executor::ThreadPool,
}

pin_project_lite::pin_project! {
    /// TODO: docs.
    pub struct ThreadPoolTask<T: 'static> {
        #[pin]
        inner: flume::r#async::RecvFut<'static, Result<T, PanicPayload>>,
        is_forever_pending: bool,
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
        let (tx, rx) = flume::bounded(1);

        self.inner.spawn_ok(async move {
            let result = panic::AssertUnwindSafe(future).catch_unwind().await;
            // The task might've been dropped, and that's ok.
            let _ = tx.send(result);
        });

        ThreadPoolTask {
            inner: rx.into_recv_async(),
            is_forever_pending: false,
        }
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
            Ok(Ok(value)) => Poll::Ready(value),
            Ok(Err(panic_payload)) => panic::resume_unwind(panic_payload),
            Err(flume::RecvError::Disconnected) => {
                // This can happen if all handles to the thread pool are
                // dropped.
                *this.is_forever_pending = true;
                Poll::Pending
            },
        }
    }
}
