use core::cell::OnceCell;
use core::future::Future;
use std::sync::Arc;

use async_task::{Builder, Runnable};
use concurrent_queue::{ConcurrentQueue, PopError, PushError};
use nvim_oxi::{libuv, schedule};

use crate::join_handle::JoinHandle;

thread_local! {
    static LOCAL_EXECUTOR: LocalExecutor = const { LocalExecutor::new() };
}

/// TODO: doc
pub(super) fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + 'static,
    F::Output: 'static,
{
    LOCAL_EXECUTOR.with(|executor| executor.spawn(future))
}

/// TODO: docs
#[derive(Default)]
struct LocalExecutor {
    inner: OnceCell<LocalExecutorInner>,
}

impl LocalExecutor {
    /// TODO: docs
    fn inner(&self) -> &LocalExecutorInner {
        self.inner.get_or_init(LocalExecutorInner::new)
    }

    /// TODO: docs
    const fn new() -> Self {
        Self { inner: OnceCell::new() }
    }

    /// TODO: docs
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.inner().spawn(future)
    }
}

/// TODO: docs
struct LocalExecutorInner {
    /// TODO: docs
    async_handle: libuv::AsyncHandle,

    /// TODO: docs
    state: Arc<LocalExecutorState>,
}

impl LocalExecutorInner {
    /// TODO: docs
    fn new() -> Self {
        let state = Arc::new(LocalExecutorState::new());

        let also_state = Arc::clone(&state);

        // This callback will be registered to be executed on the next tick of
        // the libuv event loop everytime a future calls `Waker::wake()`.
        let async_handle = libuv::AsyncHandle::new(move || {
            let state = Arc::clone(&also_state);

            // TODO: explain why we schedule this.
            schedule(move |()| {
                state.tick_all();
            });
        })
        .expect("creating an async handle never fails");

        Self { async_handle, state }
    }

    /// TODO: docs
    fn schedule(&self) -> impl Fn(Runnable<()>) + Send + Sync + 'static {
        let async_handle = self.async_handle.clone();

        let state = Arc::clone(&self.state);

        move |runnable| {
            let task = Task::new(runnable);
            state.woken_queue.push_back(task);
            async_handle.send().expect("sending an async handle never fails");
        }
    }

    /// TODO: docs
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        let builder = Builder::new().propagate_panic(true);

        // SAFETY:
        //
        // - the future is not `Send`, but we're dropping the `Runnable` on the
        // next line, so definitely on this thread;
        let (runnable, task) =
            unsafe { builder.spawn_unchecked(|()| future, self.schedule()) };

        // Poll the future once immediately.
        Task::new(runnable).poll();

        JoinHandle::new(task)
    }
}

/// TODO: docs
struct LocalExecutorState {
    woken_queue: TaskQueue,
}

impl LocalExecutorState {
    /// TODO: docs
    fn new() -> Self {
        Self { woken_queue: TaskQueue::new() }
    }

    /// TODO: docs
    fn tick_all(&self) {
        for _ in 0..self.woken_queue.len() {
            self.woken_queue.pop_front().expect("checked queue length").poll();
        }
    }
}

/// TODO: docs
struct TaskQueue {
    queue: ConcurrentQueue<Task>,
}

impl TaskQueue {
    /// TODO: docs
    fn len(&self) -> usize {
        self.queue.len()
    }

    /// TODO: docs
    fn new() -> Self {
        Self { queue: ConcurrentQueue::unbounded() }
    }

    /// TODO: docs
    fn pop_front(&self) -> Option<Task> {
        match self.queue.pop() {
            Ok(task) => Some(task),
            Err(PopError::Empty) => None,
            Err(PopError::Closed) => unreachable!(),
        }
    }

    /// TODO: docs
    fn push_back(&self, task: Task) {
        match self.queue.push(task) {
            Ok(()) => {},
            Err(PushError::Full(_)) => unreachable!("queue is unbounded"),
            Err(PushError::Closed(_)) => unreachable!("queue is never closed"),
        }
    }
}

/// TODO: docs
struct Task {
    runnable: Runnable<()>,
}

impl Task {
    fn new(runnable: Runnable<()>) -> Self {
        Self { runnable }
    }

    fn poll(self) {
        self.runnable.run();
    }
}
