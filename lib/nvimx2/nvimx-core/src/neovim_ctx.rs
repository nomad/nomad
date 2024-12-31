use crate::backend_handle::BackendMut;

/// TODO: docs.
pub struct NeovimCtx<'a, B> {
    backend: BackendMut<'a, B>,
}

impl<'a, B> NeovimCtx<'a, B> {
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
}
