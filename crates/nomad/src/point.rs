use crate::ByteOffset;

/// The 2D equivalent of a `ByteOffset`.
#[derive(Copy, Clone, PartialEq)]
pub struct Point {
    /// The index of the line in the buffer.
    pub(crate) line_idx: usize,

    /// The byte offset in the line.
    pub(crate) byte_offset: ByteOffset,
}

impl Point {
    pub(super) fn zero() -> Self {
        Self { line_idx: 0, byte_offset: ByteOffset::new(0) }
    }
}
