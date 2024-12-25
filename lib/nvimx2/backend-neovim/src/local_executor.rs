use nvimx_core::executor::LocalExecutor;

/// TODO: docs.
pub struct NeovimLocalExecutor;

impl LocalExecutor for NeovimLocalExecutor {
    type Task<T> = core::future::Ready<T>;

    #[inline]
    fn spawn<Fut>(&mut self, _fut: Fut) -> Self::Task<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        todo!();
    }
}
