use core::pin::Pin;
use core::task::{Context, Poll};

use async_task::Runnable;
use ed_core::backend::{self, BackgroundExecutor, LocalExecutor};
use flume::{Receiver, Sender};
use futures_lite::future::{self, FutureExt};

pub struct Executor {
    runner: Option<Runner>,
    spawner: Spawner,
}

pin_project_lite::pin_project! {
    pub struct Task<T> {
        #[pin]
        inner: async_task::Task<T>,
    }
}

#[derive(Clone)]
pub(crate) struct Spawner {
    runnable_tx: Sender<Runnable>,
}

pub(crate) struct Runner {
    runnable_rx: Receiver<Runnable>,
}

impl Executor {
    pub(crate) fn take_runner(&mut self) -> Option<Runner> {
        self.runner.take()
    }
}

impl Runner {
    pub(crate) async fn run<Fut: Future>(
        &self,
        future: Fut,
        run_all: bool,
    ) -> Fut::Output {
        let keep_polling_runnables = async {
            while let Ok(runnable) = self.runnable_rx.recv_async().await {
                runnable.run();
            }
        };
        if run_all {
            let (out, ()) = future::zip(future, keep_polling_runnables).await;
            out
        } else {
            future
                .or(async move {
                    keep_polling_runnables.await;
                    unreachable!("future will always complete first");
                })
                .await
        }
    }
}

impl Spawner {
    fn schedule(&self) -> impl Fn(Runnable) + Send + Sync + 'static {
        let runnable_tx = self.runnable_tx.clone();
        move |runnable| {
            let _ = runnable_tx.send(runnable);
        }
    }

    fn spawn_background<Fut>(&self, fut: Fut) -> Task<Fut::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let (runnable, task) = async_task::spawn(fut, self.schedule());
        runnable.schedule();
        Task { inner: task }
    }

    fn spawn_local<Fut>(&self, fut: Fut) -> Task<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        let (runnable, task) = async_task::spawn_local(fut, self.schedule());
        runnable.schedule();
        Task { inner: task }
    }
}

impl LocalExecutor for Executor {
    type Task<T> = Task<T>;

    fn spawn<Fut>(&mut self, fut: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        self.spawner.spawn_local(fut)
    }
}

impl BackgroundExecutor for Executor {
    type Task<T> = Task<T>;

    fn spawn<Fut>(&mut self, fut: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        self.spawner.spawn_background(fut)
    }
}

impl Clone for Executor {
    fn clone(&self) -> Self {
        Self { runner: None, spawner: self.spawner.clone() }
    }
}

impl AsRef<Self> for Executor {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsMut<Self> for Executor {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl Default for Executor {
    fn default() -> Self {
        let (runnable_tx, runnable_rx) = flume::unbounded();
        Self {
            runner: Some(Runner { runnable_rx }),
            spawner: Spawner { runnable_tx },
        }
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        self.project().inner.poll(ctx)
    }
}

impl<T> backend::Task<T> for Task<T> {
    fn detach(self) {
        self.inner.detach();
    }
}
