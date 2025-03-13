use std::borrow::Cow;

use crate::ByteOffset;

/// TODO: docs.
pub trait Buffer {
    /// TODO: docs.
    type Id: Clone;

    /// TODO: docs.
    fn byte_len(&self) -> ByteOffset;

    /// TODO: docs.
    fn id(&self) -> Self::Id;

    /// TODO: docs.
    fn name(&self) -> Cow<'_, str>;
}

impl<Buf: Buffer> Buffer for &mut Buf {
    type Id = Buf::Id;

    #[inline]
    fn byte_len(&self) -> ByteOffset {
        Buf::byte_len(self)
    }

    #[inline]
    fn id(&self) -> Self::Id {
        Buf::id(self)
    }

    #[inline]
    fn name(&self) -> Cow<'_, str> {
        Buf::name(self)
    }
}
