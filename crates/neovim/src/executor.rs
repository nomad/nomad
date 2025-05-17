//! TODO: docs.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::rc::Rc;

use async_task::Builder;
use concurrent_queue::{ConcurrentQueue, PopError, PushError};
use ed::executor::{Executor, LocalSpawner, Runner, Task};
use thread_pool::ThreadPool;

use crate::oxi;

type Runnable = async_task::Runnable<()>;

/// TODO: docs.
#[derive(Default)]
pub struct NeovimExecutor {
    runner: NeovimRunner,
    local_spawner: NeovimLocalSpawner,
    background_spawner: ThreadPool,
}

/// TODO: docs.
#[derive(Default, Copy, Clone)]
pub struct NeovimRunner;

/// TODO: docs.
#[derive(Clone)]
pub struct NeovimLocalSpawner {
    /// TODO: docs
    async_handle: oxi::libuv::AsyncHandle,

    /// TODO: docs
    runnable_queue: Rc<RunnableQueue>,
}

pin_project_lite::pin_project! {
    /// TODO: docs.
    pub struct NeovimLocalTask<T> {
        #[pin]
        inner: async_task::Task<T>,
    }
}

/// The queue of runnables that are ready to be polled.
struct RunnableQueue {
    inner: ConcurrentQueue<Runnable>,
}

impl NeovimLocalSpawner {
    #[inline]
    fn init() -> Self {
        let runnable_queue = Rc::new(RunnableQueue::new());

        // This callback will be registered to be executed on the next tick of
        // the libuv event loop everytime a future wakes its `Waker`.
        let async_handle = {
            let runnable_queue = Rc::clone(&runnable_queue);

            oxi::libuv::AsyncHandle::new(move || {
                let runnable_queue = Rc::clone(&runnable_queue);

                // We schedule the poll to avoid `textlock` and other
                // synchronization issues.
                oxi::schedule(move |()| {
                    for _ in 0..runnable_queue.len() {
                        runnable_queue
                            .pop_front()
                            .expect("checked queue length")
                            .run();
                    }
                });
            })
        }
        .expect("creating an async handle never fails");

        Self { async_handle, runnable_queue }
    }
}

impl RunnableQueue {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    fn new() -> Self {
        Self { inner: ConcurrentQueue::unbounded() }
    }

    #[inline]
    fn pop_front(&self) -> Option<Runnable> {
        match self.inner.pop() {
            Ok(runnable) => Some(runnable),
            Err(PopError::Empty) => None,
            Err(PopError::Closed) => unreachable!(),
        }
    }

    #[inline]
    fn push_back(&self, runnable: Runnable) {
        match self.inner.push(runnable) {
            Ok(()) => {},
            Err(PushError::Full(_)) => unreachable!("queue is unbounded"),
            Err(PushError::Closed(_)) => unreachable!("queue is never closed"),
        }
    }
}

impl Executor for NeovimExecutor {
    type Runner = NeovimRunner;
    type LocalSpawner = NeovimLocalSpawner;
    type BackgroundSpawner = ThreadPool;

    #[inline]
    fn runner(&mut self) -> &mut Self::Runner {
        &mut self.runner
    }

    #[inline]
    fn local_spawner(&mut self) -> &mut Self::LocalSpawner {
        &mut self.local_spawner
    }

    #[inline]
    fn background_spawner(&mut self) -> &mut Self::BackgroundSpawner {
        &mut self.background_spawner
    }
}

impl Runner for NeovimRunner {
    #[inline]
    async fn run<T>(&mut self, future: impl Future<Output = T>) -> T {
        // Scheduling a task also notifies the libuv event loop, so we don't
        // have to do anything else here.
        future.await
    }
}

impl Default for NeovimLocalSpawner {
    #[inline]
    fn default() -> Self {
        Self::init()
    }
}

impl LocalSpawner for NeovimLocalSpawner {
    type Task<T> = NeovimLocalTask<T>;

    #[inline]
    fn spawn<Fut>(&mut self, future: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        let builder = Builder::new().propagate_panic(true);

        let schedule = {
            let this = self.clone();
            move |runnable| {
                this.runnable_queue.push_back(runnable);
                this.async_handle
                    .send()
                    .expect("sending an async handle never fails");
            }
        };

        // SAFETY:
        //
        // - the future is not `Send`, but we're dropping the `Runnable` on the
        // next line, so definitely on this thread;
        let (runnable, task) =
            unsafe { builder.spawn_unchecked(|()| future, schedule) };

        // Poll the future once immediately.
        runnable.run();

        NeovimLocalTask { inner: task }
    }
}

impl<T> Task<T> for NeovimLocalTask<T> {
    #[inline]
    fn detach(self) {
        self.inner.detach()
    }
}

impl<T> Future for NeovimLocalTask<T> {
    type Output = T;

    #[inline]
    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<T> {
        self.project().inner.poll(ctx)
    }
}
