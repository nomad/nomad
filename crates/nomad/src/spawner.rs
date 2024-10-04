use core::future::Future;

/// TODO: docs.
pub trait Spawner {
    /// TODO: docs.
    type JoinHandle<T>: JoinHandle<T>;

    /// TODO: docs.
    fn spawn<F>(&self, fut: F) -> Self::JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static;

    /// TODO: docs.
    fn spawn_background<F>(&self, fut: F) -> Self::JoinHandle<F::Output>
    where
        F: Future + 'static + Send,
        F::Output: 'static + Send;
}

/// TODO: docs.
pub trait JoinHandle<T>: Future<Output = T> {
    /// TODO: docs.
    fn detach(self);
}
