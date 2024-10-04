use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use async_task::Task;
use pin_project_lite::pin_project;

use crate::JoinHandle;

pin_project! {
    /// TODO: docs
    pub struct NeovimJoinHandle<T> {
        #[pin]
        task: Task<T>,
    }
}

impl<T> NeovimJoinHandle<T> {
    pub(crate) fn new(task: Task<T>) -> NeovimJoinHandle<T> {
        NeovimJoinHandle { task }
    }
}

impl<T> Future for NeovimJoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<T> {
        self.project().task.poll(ctx)
    }
}

impl<T> JoinHandle<T> for NeovimJoinHandle<T> {
    fn detach(self) {
        self.task.detach()
    }
}
