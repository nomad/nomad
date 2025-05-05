use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::rc::Rc;

use async_task::Builder;
use concurrent_queue::{ConcurrentQueue, PopError, PushError};
use ed::backend::{LocalExecutor, Task};

use crate::oxi::{self, libuv};

type Runnable = async_task::Runnable<()>;

/// TODO: docs.
#[derive(Clone)]
pub struct NeovimLocalExecutor {
    /// TODO: docs
    async_handle: libuv::AsyncHandle,

    /// TODO: docs
    state: Rc<ExecutorState>,
}

pin_project_lite::pin_project! {
    /// TODO: docs.
    pub struct NeovimLocalTask<T> {
        #[pin]
        inner: async_task::Task<T>,
    }
}

struct ExecutorState {
    woken_queue: RunnableQueue,
}

/// The queue of runnables that are ready to be polled.
struct RunnableQueue {
    queue: ConcurrentQueue<Runnable>,
}

impl NeovimLocalExecutor {
    /// TODO: docs
    #[inline]
    pub fn init() -> Self {
        let state = Rc::new(ExecutorState::new());

        let also_state = Rc::clone(&state);

        // This callback will be registered to be executed on the next tick of
        // the libuv event loop everytime a future wakes its `Waker`.
        let async_handle = libuv::AsyncHandle::new(move || {
            let state = Rc::clone(&also_state);

            // We schedule the poll to avoid `textlock` and other
            // synchronization issues.
            oxi::schedule(move |()| state.tick_all());
        })
        .expect("creating an async handle never fails");

        Self { async_handle, state }
    }

    #[inline]
    fn spawn_inner<F>(&self, future: F) -> NeovimLocalTask<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        let builder = Builder::new().propagate_panic(true);

        let schedule = {
            let async_handle = self.async_handle.clone();
            let state = Rc::clone(&self.state);
            move |runnable| {
                state.woken_queue.push_back(runnable);
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
        runnable.run();

        NeovimLocalTask { inner: task }
    }
}

impl ExecutorState {
    #[inline]
    fn new() -> Self {
        Self { woken_queue: RunnableQueue::new() }
    }

    /// Polls all the runnables in the queue.
    #[inline]
    fn tick_all(&self) {
        for _ in 0..self.woken_queue.len() {
            self.woken_queue.pop_front().expect("checked queue length").run();
        }
    }
}

impl RunnableQueue {
    #[inline]
    fn len(&self) -> usize {
        self.queue.len()
    }

    #[inline]
    fn new() -> Self {
        Self { queue: ConcurrentQueue::unbounded() }
    }

    #[inline]
    fn pop_front(&self) -> Option<Runnable> {
        match self.queue.pop() {
            Ok(runnable) => Some(runnable),
            Err(PopError::Empty) => None,
            Err(PopError::Closed) => unreachable!(),
        }
    }

    #[inline]
    fn push_back(&self, runnable: Runnable) {
        match self.queue.push(runnable) {
            Ok(()) => {},
            Err(PushError::Full(_)) => unreachable!("queue is unbounded"),
            Err(PushError::Closed(_)) => unreachable!("queue is never closed"),
        }
    }
}

impl LocalExecutor for NeovimLocalExecutor {
    type Task<T> = NeovimLocalTask<T>;

    #[inline]
    fn spawn<Fut>(&mut self, future: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        self.spawn_inner(future)
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
