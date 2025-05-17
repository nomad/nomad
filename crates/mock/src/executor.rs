use core::pin::Pin;
use core::task::{Context, Poll};
use std::rc::Rc;

use async_task::Runnable;
use ed::executor::{self, BackgroundSpawner, LocalSpawner};
use futures_lite::future::{self, FutureExt};

pub struct Executor<BackgroundSpawner = Spawner> {
    runner: Runner,
    local_spawner: Spawner,
    background_spawner: BackgroundSpawner,
}

pin_project_lite::pin_project! {
    pub struct Task<T> {
        #[pin]
        inner: async_task::Task<T>,
    }
}

#[derive(Clone)]
pub struct Runner {
    runnable_rx: Rc<flume::Receiver<Runnable>>,
}

#[derive(Clone)]
pub struct Spawner {
    runnable_tx: flume::Sender<Runnable>,
}

impl<BgSpawner> Executor<BgSpawner> {
    pub(crate) fn with_background_spawner<NewBgSpawner>(
        self,
        spawner: NewBgSpawner,
    ) -> Executor<NewBgSpawner> {
        Executor {
            runner: self.runner,
            local_spawner: self.local_spawner,
            background_spawner: spawner,
        }
    }
}

impl Runner {
    pub(crate) async fn run_inner<Fut: Future>(
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

impl Default for Executor {
    fn default() -> Self {
        let (runnable_tx, runnable_rx) = flume::unbounded();
        let runner = Runner { runnable_rx: Rc::new(runnable_rx) };
        let spawner = Spawner { runnable_tx };
        Self {
            runner,
            local_spawner: spawner.clone(),
            background_spawner: spawner,
        }
    }
}

impl<BgSpawner: BackgroundSpawner> executor::Executor for Executor<BgSpawner> {
    type Runner = Runner;
    type LocalSpawner = Spawner;
    type BackgroundSpawner = BgSpawner;

    fn runner(&mut self) -> &mut Self::Runner {
        &mut self.runner
    }

    fn local_spawner(&mut self) -> &mut Self::LocalSpawner {
        &mut self.local_spawner
    }

    fn background_spawner(&mut self) -> &mut Self::BackgroundSpawner {
        &mut self.background_spawner
    }
}

impl AsMut<Self> for Runner {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl executor::Runner for Runner {
    async fn run<T>(&mut self, future: impl Future<Output = T>) -> T {
        self.run_inner(future, false).await
    }
}

impl LocalSpawner for Spawner {
    type Task<T> = Task<T>;

    fn spawn<Fut>(&mut self, fut: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        self.spawn_local(fut)
    }
}

impl BackgroundSpawner for Spawner {
    type Task<T: Send + 'static> = Task<T>;

    fn spawn<Fut>(&mut self, fut: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        self.spawn_background(fut)
    }
}

impl AsRef<Self> for Executor {
    fn as_ref(&self) -> &Self {
        self
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

impl<T> executor::Task<T> for Task<T> {
    fn detach(self) {
        self.inner.detach();
    }
}
