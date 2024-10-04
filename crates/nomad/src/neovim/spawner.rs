use core::future::Future;

use super::{executor, NeovimJoinHandle};
use crate::Spawner;

/// TODO: docs.
#[derive(Debug, Copy, Clone, Default)]
pub struct NeovimSpawner;

impl Spawner for NeovimSpawner {
    type JoinHandle<T> = NeovimJoinHandle<T>;

    fn spawn<F>(&self, fut: F) -> Self::JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        executor::spawn(fut)
    }

    fn spawn_background<F>(&self, fut: F) -> Self::JoinHandle<F::Output>
    where
        F: Future + 'static + Send,
        F::Output: 'static + Send,
    {
        self.spawn(fut)
    }
}
