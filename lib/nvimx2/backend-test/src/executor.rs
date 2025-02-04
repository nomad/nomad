use core::pin::Pin;
use core::task::{Context, Poll};

use async_task::Runnable;
use flume::{Receiver, Sender};
use futures_lite::FutureExt;
use nvimx_core::backend::{BackgroundExecutor, LocalExecutor, Task};

#[derive(Clone)]
pub struct TestExecutor {
    runnable_tx: Sender<Runnable>,
    runnable_rx: Receiver<Runnable>,
}

pin_project_lite::pin_project! {
    pub struct TestTask<T> {
        #[pin]
        inner: async_task::Task<T>,
    }
}

impl TestExecutor {
    pub fn block_on<T>(&self, future: impl Future<Output = T>) -> T {
        futures_lite::future::block_on(self.run(future))
    }

    pub async fn run<T>(&self, future: impl Future<Output = T>) -> T {
        let keep_polling_runnables = async move {
            loop {
                self.runnable_rx
                    .recv_async()
                    .await
                    .expect("Self has a sender")
                    .run();
            }
        };
        future.or(keep_polling_runnables).await
    }

    fn schedule(&self) -> impl Fn(Runnable) + Send + Sync + 'static {
        let runnable_tx = self.runnable_tx.clone();
        move |runnable| {
            let _ = runnable_tx.send(runnable);
        }
    }
}

impl LocalExecutor for TestExecutor {
    type Task<T> = TestTask<T>;

    fn spawn<Fut>(&mut self, fut: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        let (runnable, task) = async_task::spawn_local(fut, self.schedule());
        runnable.schedule();
        TestTask { inner: task }
    }
}

impl BackgroundExecutor for TestExecutor {
    type Task<T> = TestTask<T>;

    fn spawn<Fut>(&mut self, fut: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let (runnable, task) = async_task::spawn(fut, self.schedule());
        runnable.schedule();
        TestTask { inner: task }
    }
}

impl Default for TestExecutor {
    fn default() -> Self {
        let (runnable_tx, runnable_rx) = flume::unbounded();
        Self { runnable_tx, runnable_rx }
    }
}

impl<T> Future for TestTask<T> {
    type Output = T;

    fn poll(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        self.project().inner.poll(ctx)
    }
}

impl<T> Task<T> for TestTask<T> {
    fn detach(self) {
        self.inner.detach();
    }
}
