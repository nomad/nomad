use super::GetCtx;

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
    pub fn get<'a>(&'a self, ctx: &'a GetCtx) -> &'a T {
        self.inner.get(ctx.as_engine())
    }

    #[inline]
    pub(crate) fn new(inner: pond::Get<T>) -> Self {
        Self { inner }
    }
}
