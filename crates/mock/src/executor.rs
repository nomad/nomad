use std::rc::Rc;

use async_task::Runnable;
use ed::executor::{self, BackgroundSpawner, LocalSpawner};
use futures_lite::future::{self, FutureExt};

pub struct Executor<BackgroundSpawner = Spawner> {
    pub(crate) runner: Runner,
    local_spawner: Spawner,
    background_spawner: BackgroundSpawner,
}

#[derive(Clone)]
pub struct Spawner {
    runnable_tx: flume::Sender<Runnable>,
}

#[derive(Clone)]
pub(crate) struct Runner {
    runnable_rx: Rc<flume::Receiver<Runnable>>,
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

    fn spawn_background<Fut>(&self, fut: Fut) -> async_task::Task<Fut::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let (runnable, task) = async_task::spawn(fut, self.schedule());
        runnable.schedule();
        task
    }

    fn spawn_local<Fut>(&self, fut: Fut) -> async_task::Task<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        let (runnable, task) = async_task::spawn_local(fut, self.schedule());
        runnable.schedule();
        task
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
    type LocalSpawner = Spawner;
    type BackgroundSpawner = BgSpawner;

    fn run<Fut: Future>(
        &mut self,
        future: Fut,
    ) -> impl Future<Output = Fut::Output> + use<BgSpawner, Fut> {
        let runner = self.runner.clone();
        async move { runner.run(future, false).await }
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

impl LocalSpawner for Spawner {
    type Task<T> = async_task::Task<T>;

    fn spawn<Fut>(&mut self, fut: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        self.spawn_local(fut)
    }
}

impl BackgroundSpawner for Spawner {
    type Task<T: Send + 'static> = async_task::Task<T>;

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
