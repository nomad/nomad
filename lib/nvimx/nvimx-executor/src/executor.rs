use core::future::Future;
use std::rc::Rc;

use async_task::{Builder, Runnable};
use concurrent_queue::{ConcurrentQueue, PopError, PushError};
use nvim_oxi::{libuv, schedule};

use crate::join_handle::JoinHandle;

/// TODO: docs
pub struct Executor {
    /// TODO: docs
    async_handle: libuv::AsyncHandle,

    /// TODO: docs
    state: Rc<ExecutorState>,
}

struct ExecutorState {
    woken_queue: TaskQueue,
}

/// The queue of tasks that are ready to be polled.
struct TaskQueue {
    queue: ConcurrentQueue<Task>,
}

struct Task {
    runnable: Runnable<()>,
}

impl Executor {
    /// TODO: docs
    pub fn register() -> Self {
        let state = Rc::new(ExecutorState::new());

        let also_state = Rc::clone(&state);

        // This callback will be registered to be executed on the next tick of
        // the libuv event loop everytime a future calls `Waker::wake()`.
        let async_handle = libuv::AsyncHandle::new(move || {
            let state = Rc::clone(&also_state);

            // We schedule the poll to avoid `textlock` and other
            // synchronization issues.
            schedule(move |()| {
                state.tick_all();
            });
        })
        .expect("creating an async handle never fails");

        Self { async_handle, state }
    }

    /// TODO: docs
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        let builder = Builder::new().propagate_panic(true);

        let schedule = {
            let async_handle = self.async_handle.clone();
            let state = Rc::clone(&self.state);
            move |runnable| {
                let task = Task::new(runnable);
                state.woken_queue.push_back(task);
                async_handle
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
        Task::new(runnable).poll();

        JoinHandle::new(task)
    }
}

impl ExecutorState {
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

impl Task {
    fn new(runnable: Runnable<()>) -> Self {
        Self { runnable }
    }

    fn poll(self) {
        self.runnable.run();
    }
}
