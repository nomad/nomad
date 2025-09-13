use crate::Task;

/// TODO: docs.
pub trait BackgroundSpawner: Clone + Send {
    /// TODO: docs.
    type Task<T: Send + 'static>: Task<T> + Send;

    /// TODO: docs.
    fn spawn<Fut>(&self, fut: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static;
}
