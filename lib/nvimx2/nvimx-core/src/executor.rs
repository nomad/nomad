//! TODO: docs.

use crate::Backend;

/// TODO: docs.
pub type TaskLocal<T, B> =
    <<B as Backend>::LocalExecutor as LocalExecutor>::Task<T>;

/// TODO: docs.
pub type TaskBackground<T, B> =
    <<B as Backend>::BackgroundExecutor as BackgroundExecutor>::Task<T>;

/// TODO: docs.
pub trait LocalExecutor {
    /// TODO: docs.
    type Task<T>: Future<Output = T>;

    /// TODO: docs.
    fn spawn<Fut>(&mut self, f: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static;
}

/// TODO: docs.
pub trait BackgroundExecutor {
    /// TODO: docs.
    type Task<T>: Future<Output = T>;

    /// TODO: docs.
    fn spawn<Fut>(&mut self, f: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + Send + Sync + 'static,
        Fut::Output: Send + Sync + 'static;
}
