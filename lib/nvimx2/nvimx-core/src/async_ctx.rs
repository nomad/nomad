use core::marker::PhantomData;

use crate::executor::TaskLocal;
use crate::{Backend, BackendHandle, NeovimCtx};

/// TODO: docs.
pub struct AsyncCtx<'a, B> {
    backend: BackendHandle<B>,
    _non_static: PhantomData<&'a ()>,
}

impl<'a, B> AsyncCtx<'a, B>
where
    B: Backend,
{
    /// TODO: docs.
    #[inline]
    pub fn with_ctx<F, R>(&self, f: F) -> TaskLocal<R, B>
    where
        F: FnOnce(&mut NeovimCtx<B>) -> R,
    {
        todo!();
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn new(backend: BackendHandle<B>) -> Self {
        Self { backend, _non_static: PhantomData }
    }
}
