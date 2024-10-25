use core::ops::Range;

use nvim_oxi::api::opts;

use crate::ctx::TextBufferCtx;
use crate::point::Point;
use crate::{ByteOffset, Text};

/// TODO: docs.
#[derive(Clone)]
pub struct Replacement {
    deleted_range: Range<ByteOffset>,
    inserted_text: Text,
}

impl Replacement {
    /// Returns the range of bytes that were deleted.
    pub fn deleted_range(&self) -> Range<ByteOffset> {
        self.deleted_range.clone()
    }

    /// Returns the text that was inserted.
    pub fn inserted_text(&self) -> &Text {
        &self.inserted_text
    }

    pub(crate) fn from_on_bytes_args(
        args: opts::OnBytesArgs,
        ctx: TextBufferCtx<'_>,
    ) -> Self {
        let (
            _bytes,
            buf,
            _changedtick,
            start_row,
            start_col,
            start_offset,
            _old_end_row,
            _old_end_col,
            old_end_len,
            new_end_row,
            new_end_col,
            _new_end_len,
        ) = args;

        debug_assert_eq!(buf, ctx.buffer_id().as_nvim());

        let deleted_range =
            (start_offset).into()..(start_offset + old_end_len).into();

        let start =
            Point { line_idx: start_row, byte_offset: start_offset.into() };

        let end = Point {
            line_idx: start_row + new_end_row,
            byte_offset: (start_col * (new_end_row == 0) as usize
                + new_end_col)
                .into(),
        };

        let inserted_text = if start == end {
            Text::new()
        } else {
            ctx.get_text_in_point_range(start..end)
        };

        Self { deleted_range, inserted_text }
    }

    pub(crate) fn new(
        deleted_range: Range<ByteOffset>,
        inserted_text: Text,
    ) -> Self {
        Self { deleted_range, inserted_text }
    }
}

impl From<e31e::Hunk> for Replacement {
    fn from(hunk: e31e::Hunk) -> Self {
        let deleted_start = hunk.removed_range.start.into();
        let deleted_end = hunk.removed_range.start.into();
        let mut inserted_text = Text::new();
        inserted_text.push_str(hunk.inserted_text.as_str());
        Self { deleted_range: deleted_start..deleted_end, inserted_text }
    }
}

impl From<Replacement> for e31e::Hunk {
    fn from(replacement: Replacement) -> Self {
        let removed_start = replacement.deleted_range.start.into_u64();
        let removed_end = replacement.deleted_range.end.into_u64();
        Self {
            removed_range: removed_start..removed_end,
            inserted_text: e31e::Text::new(replacement.inserted_text),
        }
    }
}
