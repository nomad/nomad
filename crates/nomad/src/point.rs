use core::ops::Range;

use crop::Rope;

use crate::{ByteOffset, FromCtx, IntoCtx};

/// A point in a text buffer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Point<Offset> {
    /// The index of the line the point is on.
    line_idx: usize,

    /// The offset of the point within the line.
    line_offset: Offset,
}

impl<Offset: Copy> Point<Offset> {
    /// The index of the line the point is on.
    #[inline]
    pub fn line(&self) -> usize {
        self.line_idx
    }

    /// Creates a new [`Point`].
    #[inline]
    pub fn new(line_idx: usize, line_offset: Offset) -> Self {
        Self { line_idx, line_offset }
    }

    /// The offset of the point within the line.
    #[inline]
    pub fn offset(&self) -> Offset {
        self.line_offset
    }
}

impl FromCtx<ByteOffset, Rope> for Point<ByteOffset> {
    #[inline]
    fn from_ctx(offset: ByteOffset, rope: &Rope) -> Self {
        let offset = offset.into();
        let line = rope.line_of_byte(offset);
        let line_offset = rope.byte_of_line(line);
        let col = offset - line_offset;
        Point::new(line, ByteOffset::new(col))
    }
}

impl<Offset, Ctx> FromCtx<Range<Offset>, Ctx> for Range<Point<Offset>>
where
    Offset: IntoCtx<Point<Offset>, Ctx>,
{
    #[inline]
    fn from_ctx(range: Range<Offset>, ctx: &Ctx) -> Self {
        let start = range.start.into_ctx(ctx);
        let end = range.end.into_ctx(ctx);
        start..end
    }
}
