use super::ctx;

/// TODO: docs
pub struct Get<T> {
    inner: pond::Get<T>,
}

impl<T> Clone for Get<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T> Get<T> {
    /// TODO: docs
    #[inline]
    pub fn get(&self) -> &T {
        ctx::get(self)
    }

    /// TODO: docs
    #[inline]
    pub(super) fn inner(&self) -> &pond::Get<T> {
        &self.inner
    }

    #[inline]
    pub(super) fn new(inner: pond::Get<T>) -> Self {
        Self { inner }
    }
}
