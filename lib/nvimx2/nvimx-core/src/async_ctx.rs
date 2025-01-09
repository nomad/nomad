use core::marker::PhantomData;

use crate::NeovimCtx;
use crate::backend::{Backend, BackendHandle};
use crate::executor::{BackgroundExecutor, TaskBackground};
use crate::notify::ModulePath;
use crate::plugin::Plugin;

/// TODO: docs.
pub struct AsyncCtx<'a, P, B> {
    backend: BackendHandle<B>,
    module_path: ModulePath,
    plugin: PhantomData<P>,
    _non_static: PhantomData<&'a ()>,
}

impl<P, B> AsyncCtx<'_, P, B>
where
    P: Plugin<B>,
    B: Backend,
{
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
        Fun: FnOnce(&mut NeovimCtx<P, B>) -> Out,
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
        Self {
            backend,
            module_path,
            plugin: PhantomData,
            _non_static: PhantomData,
        }
    }
}
