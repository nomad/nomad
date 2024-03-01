use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use async_task::Task;
use pin_project_lite::pin_project;

pin_project! {
    /// TODO: docs
    pub struct JoinHandle<T> {
        #[pin]
        task: Task<T>,
    }
}

impl<T> JoinHandle<T> {
    /// TODO: docs
    #[inline]
    pub(crate) fn new(task: Task<T>) -> JoinHandle<T> {
        JoinHandle { task }
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = T;

    #[inline]
    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<T> {
        self.project().task.poll(ctx)
    }
}
