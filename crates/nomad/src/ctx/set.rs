use super::SetCtx;

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
    pub(crate) fn new(inner: pond::Set<T>) -> Self {
        Self { inner }
    }

    /// TODO: docs
    #[inline]
    pub fn update<F>(&self, update_with: F, ctx: &mut SetCtx)
    where
        F: FnOnce(&mut T),
    {
        self.inner.update(update_with, ctx.as_engine_mut())
    }

    /// TODO: docs
    #[inline]
    pub fn set(&self, new_value: T, ctx: &mut SetCtx) {
        self.inner.set(new_value, ctx.as_engine_mut())
    }
}
