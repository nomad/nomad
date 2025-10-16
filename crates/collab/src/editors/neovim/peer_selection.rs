use core::cell::Cell;
use core::ops::Range;

use collab_types::PeerId;
use editor::ByteOffset;
use neovim::buffer::BufferExt;
use neovim::oxi::api;

use crate::editors::neovim::PeerHighlightGroup;

/// A remote peer's selection in a buffer.
pub struct NeovimPeerSelection {
    /// The buffer the selection is in.
    buffer: api::Buffer,

    /// The ID of the extmark used to display the selection.
    extmark_id: u32,

    /// The ID of the highlight group used to highlight the selection.
    hl_group_id: u32,

    /// The ID of the namespace the [`extmark_id`](Self::extmark_id) belongs
    /// to.
    namespace_id: u32,
}

/// The highlight group used to highlight a remote peer's selection.
pub(super) struct PeerSelectionHighlightGroup;

impl PeerSelectionHighlightGroup {
    thread_local! {
        static GROUP_IDS: Cell<[u32; 16]> = const { Cell::new([0; _]) };
    }
}

impl NeovimPeerSelection {
    /// Creates a new selection for the remote peer with given ID encompassing
    /// the given byte offset range in the given buffer.
    pub(super) fn create(
        peer_id: PeerId,
        mut buffer: api::Buffer,
        offset_range: Range<ByteOffset>,
        namespace_id: u32,
    ) -> Self {
        debug_assert!(offset_range.start <= offset_range.end);

        let hl_group_id = PeerSelectionHighlightGroup::group_id(peer_id);

        let selection_start = buffer.point_of_byte(offset_range.start);
        let selection_end = buffer.point_of_byte(offset_range.end);

        let opts = api::opts::SetExtmarkOpts::builder()
            .end_row(selection_end.newline_offset)
            .end_col(selection_end.byte_offset)
            .hl_group(hl_group_id)
            .build();

        let extmark_id = buffer
            .set_extmark(
                namespace_id,
                selection_start.newline_offset,
                selection_start.byte_offset,
                &opts,
            )
            .expect("couldn't set extmark");

        Self { buffer, extmark_id, hl_group_id, namespace_id }
    }

    /// Moves the selection to the given offset range.
    pub(super) fn r#move(&mut self, new_offset_range: Range<ByteOffset>) {
        let selection_start =
            self.buffer.point_of_byte(new_offset_range.start);
        let selection_end = self.buffer.point_of_byte(new_offset_range.end);

        let opts = api::opts::SetExtmarkOpts::builder()
            .id(self.extmark_id)
            .end_row(selection_end.newline_offset)
            .end_col(selection_end.byte_offset)
            .hl_group(self.hl_group_id)
            .build();

        let new_extmark_id = self
            .buffer
            .set_extmark(
                self.namespace_id,
                selection_start.newline_offset,
                selection_start.byte_offset,
                &opts,
            )
            .expect("couldn't set extmark");

        debug_assert_eq!(new_extmark_id, self.extmark_id);
    }

    /// Removes the selection from the buffer.
    pub(super) fn remove(mut self) {
        self.buffer
            .del_extmark(self.namespace_id, self.extmark_id)
            .expect("couldn't delete extmark");
    }
}

impl PeerHighlightGroup for PeerSelectionHighlightGroup {
    const NAME_PREFIX: &str = "NomadCollabPeerSelection";

    fn set_hl_opts() -> api::opts::SetHighlightOpts {
        api::opts::SetHighlightOpts::builder().link("Visual").build()
    }

    fn with_group_ids<R>(fun: impl FnOnce(&[Cell<u32>]) -> R) -> R {
        Self::GROUP_IDS.with(|ids| fun(ids.as_array_of_cells().as_slice()))
    }
}
