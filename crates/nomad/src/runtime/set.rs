use super::ctx;

/// TODO: docs
pub struct Set<T> {
    inner: pond::Set<T>,
}

impl<T> Clone for Set<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T> Set<T> {
    #[inline]
    pub(super) fn inner(&self) -> &pond::Set<T> {
        &self.inner
    }

    #[inline]
    pub(super) fn new(inner: pond::Set<T>) -> Self {
        Self { inner }
    }

    /// TODO: docs
    #[inline]
    pub fn set(&self, new_value: T) {
        ctx::set(self, new_value)
    }

    /// TODO: docs
    #[inline]
    pub fn update<F>(&self, update_with: F)
    where
        F: FnOnce(&mut T),
    {
        ctx::update(self, update_with)
    }
}
