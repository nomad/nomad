use core::cell::Cell;
use core::ops::Range;

use collab_types::Peer;
use editor::ByteOffset;
use neovim::buffer::{BufferExt, Point};
use neovim::oxi::api;

use crate::editors::neovim::PeerHighlightGroup;

/// Holds the state needed to display a remote peer's cursor in a buffer.
pub struct PeerCursor {
    /// The buffer the cursor is in.
    buffer: api::Buffer,

    /// The extmark ID of the highlight representing the cursor's head.
    cursor_extmark_id: u32,

    /// The ID of the namespace the
    /// [`cursor_extmark_id`](Self::cursor_extmark_id) belongs to.
    namespace_id: u32,

    /// The remote peer this tooltip is for.
    peer: Peer,
}

/// The highlight group used to highlight a remote peer's cursor.
pub(super) struct PeerCursorHighlightGroup;

impl PeerCursorHighlightGroup {
    thread_local! {
        static GROUP_IDS: Cell<[u32; 16]> = const { Cell::new([0; _]) };
    }
}

impl PeerCursor {
    /// Creates a new tooltip representing the given remote peer's cursor at
    /// the given byte offset in the given buffer.
    pub(super) fn create(
        peer: Peer,
        mut buffer: api::Buffer,
        cursor_offset: ByteOffset,
        namespace_id: u32,
    ) -> Self {
        let highlight_range = Self::highlight_range(&buffer, cursor_offset);

        let opts = api::opts::SetExtmarkOpts::builder()
            .end_row(highlight_range.end.newline_offset)
            .end_col(highlight_range.end.byte_offset)
            .hl_group(PeerCursorHighlightGroup::new(peer.id))
            .build();

        let cursor_extmark_id = buffer
            .set_extmark(
                namespace_id,
                highlight_range.start.newline_offset,
                highlight_range.start.byte_offset,
                &opts,
            )
            .expect("couldn't set extmark");

        Self { buffer, cursor_extmark_id, peer, namespace_id }
    }

    /// Moves the tooltip to the given offset.
    pub(super) fn r#move(&mut self, cursor_offset: ByteOffset) {
        let highlight_range =
            Self::highlight_range(&self.buffer, cursor_offset);

        let opts = api::opts::SetExtmarkOpts::builder()
            .id(self.cursor_extmark_id)
            .end_row(highlight_range.end.newline_offset)
            .end_col(highlight_range.end.byte_offset)
            .hl_group(PeerCursorHighlightGroup::new(self.peer.id))
            .build();

        let new_extmark_id = self
            .buffer
            .set_extmark(
                self.namespace_id,
                highlight_range.start.newline_offset,
                highlight_range.start.byte_offset,
                &opts,
            )
            .expect("couldn't set extmark");

        debug_assert_eq!(new_extmark_id, self.cursor_extmark_id);
    }

    /// Removes the tooltip from the buffer.
    pub(super) fn remove(mut self) {
        self.buffer
            .del_extmark(self.namespace_id, self.cursor_extmark_id)
            .expect("couldn't delete extmark");
    }

    /// Returns the [`Point`] range to be highlighted to represent the remote
    /// peer's cursor at the given byte offset.
    fn highlight_range(
        buffer: &api::Buffer,
        cursor_offset: ByteOffset,
    ) -> Range<Point> {
        debug_assert!(cursor_offset <= buffer.num_bytes());

        let mut highlight_start = buffer.point_of_byte(cursor_offset);

        let is_cursor_at_eol = buffer
            .num_bytes_in_line_after(highlight_start.newline_offset)
            == highlight_start.byte_offset;

        if is_cursor_at_eol {
            // If the cursor is after the uneditable eol, set the start
            // position to the end of the previous line.
            if cursor_offset == buffer.num_bytes()
                && buffer.has_uneditable_eol()
            {
                let highlight_end = highlight_start;
                highlight_start.newline_offset -= 1;
                highlight_start.byte_offset = buffer
                    .num_bytes_in_line_after(highlight_start.newline_offset);
                return highlight_start..highlight_end;
            }
        }

        let highlight_end =
            // If the cursor is at the end of the line, we set the end of the
            // highlighted range to the start of the next line.
            //
            // Apparently this works even if the cursor is on the last line,
            // and nvim_buf_set_extmark won't complain about it.
            if is_cursor_at_eol {
                Point::new(highlight_start.newline_offset + 1, 0)
            }
            // If the cursor is in the middle of a line, we set the end of the
            // highlighted range one byte after the start.
            //
            // This works because Neovim already handles offset clamping for
            // us, so even if the grapheme to the immediate right of the cursor
            // is multi-byte, Neovim will automatically extend the highlight's
            // end to the end of the grapheme.
            else {
                Point::new(
                    highlight_start.newline_offset,
                    highlight_start.byte_offset + 1,
                )
            };

        highlight_start..highlight_end
    }
}

impl PeerHighlightGroup for PeerCursorHighlightGroup {
    const NAME_PREFIX: &str = "NomadCollabPeerCursor";

    fn set_hl_opts() -> api::opts::SetHighlightOpts {
        api::opts::SetHighlightOpts::builder().link("Cursor").build()
    }

    fn with_group_ids<R>(fun: impl FnOnce(&[Cell<u32>]) -> R) -> R {
        Self::GROUP_IDS.with(|ids| fun(ids.as_array_of_cells().as_slice()))
    }
}
