use crate::backend_handle::BackendMut;
use crate::executor::{LocalExecutor, Task};
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
    pub fn spawn_local<Fun, Out>(&mut self, fun: Fun) -> TaskLocal<Out, B>
    where
        Fun: AsyncFnOnce(&mut AsyncCtx<B>) -> Out + 'static,
        Out: 'static,
    {
        let mut async_ctx = AsyncCtx::<'static, _>::new(self.backend.handle());
        let task = self
            .backend_mut()
            .local_executor()
            .spawn(async move { fun(&mut async_ctx).await });
        TaskLocal { inner: task }
    }
}

pub struct TaskLocal<T, B: Backend> {
    inner: <<B as Backend>::LocalExecutor as LocalExecutor>::Task<T>,
}

impl<T, B: Backend> TaskLocal<T, B> {
    /// TODO: docs.
    #[inline]
    pub fn detach(self) {
        self.inner.detach()
    }
}
