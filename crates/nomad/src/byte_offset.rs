use core::ops::{Add, AddAssign, Range, Sub, SubAssign};

use crop::Rope;

use crate::{FromCtx, IntoCtx, Point};

/// A byte offset in a buffer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct ByteOffset(usize);

impl ByteOffset {
    #[inline]
    pub(crate) fn new(offset: usize) -> Self {
        Self(offset)
    }
}

impl Add<Self> for ByteOffset {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign<Self> for ByteOffset {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub<Self> for ByteOffset {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign<Self> for ByteOffset {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl From<usize> for ByteOffset {
    #[inline]
    fn from(offset: usize) -> Self {
        Self::new(offset)
    }
}

impl From<ByteOffset> for usize {
    #[inline]
    fn from(offset: ByteOffset) -> usize {
        offset.0
    }
}

impl FromCtx<Point<ByteOffset>, Rope> for ByteOffset {
    #[inline]
    fn from_ctx(point: Point<ByteOffset>, rope: &Rope) -> Self {
        let line_offset = rope.byte_of_line(point.line());
        Self::new(line_offset) + point.offset()
    }
}

impl FromCtx<Range<Point<ByteOffset>>, Rope> for Range<ByteOffset> {
    #[inline]
    fn from_ctx(range: Range<Point<ByteOffset>>, rope: &Rope) -> Self {
        let start = range.start.into_ctx(rope);
        let end = range.end.into_ctx(rope);
        start..end
    }
}
