use core::marker::PhantomData;

use crate::NeovimCtx;
use crate::backend::{
    Backend,
    BackendHandle,
    BackgroundExecutor,
    TaskBackground,
};
use crate::notify::ModulePath;

/// TODO: docs.
pub struct AsyncCtx<'a, B> {
    backend: BackendHandle<B>,
    module_path: ModulePath,
    _non_static: PhantomData<&'a ()>,
}

impl<B: Backend> AsyncCtx<'_, B> {
    /// TODO: docs.
    #[inline]
    pub fn spawn_background<Fut>(
        &self,
        fut: Fut,
    ) -> TaskBackground<Fut::Output, B>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let task = self
            .backend
            .with_mut(|mut backend| backend.background_executor().spawn(fut));
        TaskBackground::new(task)
    }

    /// TODO: docs.
    #[inline]
    pub fn with_ctx<Fun, Out>(&self, fun: Fun) -> Out
    where
        Fun: FnOnce(&mut NeovimCtx<B>) -> Out,
    {
        self.backend.with_mut(|backend| {
            let mut ctx = NeovimCtx::new(backend, &self.module_path);
            fun(&mut ctx)
        })
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn new(
        backend: BackendHandle<B>,
        module_path: ModulePath,
    ) -> Self {
        Self { backend, module_path, _non_static: PhantomData }
    }
}
