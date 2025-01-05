use crate::backend_handle::BackendMut;
use crate::executor::{
    BackgroundExecutor,
    LocalExecutor,
    Task,
    TaskBackground,
};
use crate::{AsyncCtx, Backend};

/// TODO: docs.
pub struct NeovimCtx<'a, B> {
    backend: BackendMut<'a, B>,
}

impl<'a, B: Backend> NeovimCtx<'a, B> {
    /// TODO: docs.
    #[inline]
    pub fn as_mut(&mut self) -> NeovimCtx<'_, B> {
        NeovimCtx { backend: self.backend.as_mut() }
    }

    /// TODO: docs.
    #[inline]
    pub fn backend_mut(&mut self) -> &mut B {
        self.backend.inner_mut()
    }

    #[inline]
    pub(crate) fn new(handle: BackendMut<'a, B>) -> Self {
        Self { backend: handle }
    }

    /// TODO: docs.
    #[inline]
    pub fn spawn_background<Fut>(
        &mut self,
        fut: Fut,
    ) -> TaskBackground<Fut::Output, B>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        TaskBackground::new(
            self.backend_mut().background_executor().spawn(fut),
        )
    }

    /// TODO: docs.
    #[inline]
    pub fn spawn_local<Fun>(&mut self, fun: Fun)
    where
        Fun: AsyncFnOnce(&mut AsyncCtx<B>) + 'static,
    {
        let mut async_ctx = AsyncCtx::<'static, _>::new(self.backend.handle());
        self.backend_mut()
            .local_executor()
            .spawn(async move { fun(&mut async_ctx).await })
            .detach();
    }
}
