use nvimx_core::executor::BackgroundExecutor;

/// TODO: docs.
pub struct NeovimBackgroundExecutor;

impl BackgroundExecutor for NeovimBackgroundExecutor {
    type Task<T> = core::future::Ready<T>;

    #[inline]
    fn spawn<Fut>(&mut self, _fut: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + Send + Sync + 'static,
        Fut::Output: Send + Sync + 'static,
    {
        todo!();
    }
}
