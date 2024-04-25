use core::ops::Range;

use crate::ByteOffset;

#[inline]
pub(crate) fn into_byte_range(range: Range<usize>) -> Range<ByteOffset> {
    range.start.into()..range.end.into()
}
