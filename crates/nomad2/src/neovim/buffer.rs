use alloc::borrow::Cow;
use core::cmp::Ordering;
use core::ops::{Range, RangeBounds};

use collab_fs::AbsUtf8Path;
use nvim_oxi::api::Buffer as NvimBuffer;

use super::Neovim;
use crate::{ByteOffset, Text};

/// TODO: docs.
pub struct Buffer {
    id: BufferId,
}

/// TODO: docs.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BufferId {
    inner: NvimBuffer,
}

/// The 2D equivalent of a `ByteOffset`.
struct Point {
    /// The index of the line in the buffer.
    line_idx: usize,

    /// The byte offset in the line.
    byte_offset: ByteOffset,
}

impl Buffer {
    pub(super) fn new(id: BufferId) -> Self {
        Self { id }
    }

    fn get_text_in_point_range(&self, point_range: Range<Point>) -> Text {
        todo!()
    }

    fn point_of_byte_offset(&self, byte_offset: ByteOffset) -> Point {
        todo!()
    }

    fn point_of_eof(&self) -> Point {
        todo!()
    }

    fn point_range_of_byte_range<R>(&self, byte_range: R) -> Range<Point>
    where
        R: RangeBounds<ByteOffset>,
    {
        todo!()
    }

    fn replace_text_in_point_range(
        &self,
        point_range: Range<Point>,
        replacement: &str,
    ) {
        todo!()
    }
}

impl crate::Buffer<Neovim> for Buffer {
    type Id = BufferId;

    fn byte_len(&self) -> usize {
        todo!()
    }

    fn get_text<R>(&self, byte_range: R) -> Text
    where
        R: RangeBounds<ByteOffset>,
    {
        let point_range = self.point_range_of_byte_range(byte_range);
        self.get_text_in_point_range(point_range)
    }

    fn id(&self) -> Self::Id {
        self.id.clone()
    }

    fn path(&self) -> Option<Cow<'_, AbsUtf8Path>> {
        todo!()
    }

    fn set_text<R, T>(&mut self, replaced_range: R, new_text: T)
    where
        R: RangeBounds<ByteOffset>,
        T: AsRef<str>,
    {
        let point_range = self.point_range_of_byte_range(replaced_range);
        self.replace_text_in_point_range(point_range, new_text.as_ref());
    }
}

impl BufferId {
    pub(super) fn is_of_text_buffer(&self) -> bool {
        todo!();
    }

    pub(super) fn new(inner: NvimBuffer) -> Self {
        Self { inner }
    }
}

impl PartialOrd for BufferId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BufferId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.handle().cmp(&other.inner.handle())
    }
}

impl Point {
    fn zero() -> Self {
        Self { line_idx: 0, byte_offset: ByteOffset::new(0) }
    }
}
