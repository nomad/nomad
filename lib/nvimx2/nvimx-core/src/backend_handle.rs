use core::ops::{Deref, DerefMut};

use crate::Shared;

/// TODO: docs.
pub(crate) struct BackendHandle<B> {
    inner: Shared<B>,
}

/// TODO: docs.
pub(crate) struct BackendMut<'a, B> {
    backend: &'a mut B,
    handle: &'a BackendHandle<B>,
}

impl<B> BackendHandle<B> {
    #[inline]
    pub(crate) fn new(backend: B) -> Self {
        Self { inner: Shared::new(backend) }
    }

    #[track_caller]
    #[inline]
    pub(crate) fn with_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(BackendMut<'_, B>) -> R,
    {
        self.inner.with_mut(|backend| f(BackendMut { backend, handle: self }))
    }
}

impl<B> BackendMut<'_, B> {
    #[inline]
    pub(crate) fn as_mut(&mut self) -> BackendMut<'_, B> {
        BackendMut { backend: self.backend, handle: self.handle }
    }

    #[inline]
    pub(crate) fn handle(&self) -> BackendHandle<B> {
        self.handle.clone()
    }

    #[inline]
    pub(crate) fn inner(&self) -> &B {
        self.backend
    }

    #[inline]
    pub(crate) fn inner_mut(&mut self) -> &mut B {
        self.backend
    }
}

impl<B> Clone for BackendHandle<B> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<B> Deref for BackendMut<'_, B> {
    type Target = B;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.backend
    }
}

impl<B> DerefMut for BackendMut<'_, B> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.backend
    }
}
