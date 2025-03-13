use std::borrow::Cow;

use crate::ByteOffset;
use crate::backend::{Backend, Buffer, BufferId};

/// TODO: docs.
pub struct BufferCtx<'a, B: Backend> {
    inner: B::Buffer<'a>,
}

impl<'a, B: Backend> BufferCtx<'a, B> {
    /// TODO: docs.
    #[inline]
    pub fn byte_len(&self) -> ByteOffset {
        self.inner.byte_len()
    }

    /// TODO: docs.
    #[inline]
    pub fn id(&self) -> BufferId<B> {
        self.inner.id()
    }

    /// TODO: docs.
    #[inline]
    pub fn name(&self) -> Cow<'_, str> {
        self.inner.name()
    }

    #[inline]
    pub(crate) fn new(inner: B::Buffer<'a>) -> Self {
        Self { inner }
    }
}
