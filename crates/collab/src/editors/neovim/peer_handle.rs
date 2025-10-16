use core::cell::{Cell, LazyCell};

use collab_types::{Peer, PeerHandle};
use editor::ByteOffset;
use neovim::buffer::BufferExt;
use neovim::oxi::api;

use crate::editors::neovim::PeerHighlightGroup;

thread_local! {
    /// The highlight group ID of the `Normal` highlight group.
    static NORMAL_HL_GROUP_ID: LazyCell<u32> = const { LazyCell::new(|| {
        api::get_hl_id_by_name("Normal")
            .expect("couldn't get highlight group ID for 'Normal'")
    }) };
}

/// A remote peer's handle in a buffer, displayed either directly above or
/// below their [`cursor`](crate::editors::neovim::PeerCursor).
pub struct NeovimPeerHandle {
    /// The buffer the cursor is in.
    buffer: api::Buffer,

    /// The ID of the extmark used to display the handle.
    extmark_id: u32,

    /// The ID of the highlight group used to highlight the handle.
    hl_group_id: u32,

    /// The ID of the namespace the [`extmark_id`](Self::extmark_id) belongs
    /// to.
    namespace_id: u32,

    /// The remote peer's handle.
    peer_handle: PeerHandle,
}

/// The highlight group used to highlight a remote peer's handle.
pub(super) struct PeerHandleHighlightGroup;

impl PeerHandleHighlightGroup {
    thread_local! {
        static GROUP_IDS: Cell<[u32; 16]> = const { Cell::new([0; _]) };
    }
}

impl NeovimPeerHandle {
    /// Creates a new handle for the given remote peer to be displayed above or
    /// below the cursor at the given offset in the given buffer.
    pub(super) fn create(
        peer: Peer,
        mut buffer: api::Buffer,
        cursor_offset: ByteOffset,
        namespace_id: u32,
    ) -> Self {
        let hl_group_id = PeerHandleHighlightGroup::group_id(peer.id);

        let (line, mut opts_builder) = Self::extmark_params(
            buffer.clone(),
            cursor_offset,
            &peer.handle,
            hl_group_id,
        );

        let extmark_id = buffer
            .set_extmark(namespace_id, line, 0, &opts_builder.build())
            .expect("couldn't create extmark");

        Self {
            buffer,
            extmark_id,
            hl_group_id,
            namespace_id,
            peer_handle: peer.handle,
        }
    }

    /// Moves the handle to keep it in sync with the new cursor offset.
    pub(super) fn r#move(&mut self, new_cursor_offset: ByteOffset) {
        let (line, mut opts_builder) = Self::extmark_params(
            self.buffer.clone(),
            new_cursor_offset,
            &self.peer_handle,
            self.hl_group_id,
        );

        let opts = opts_builder.id(self.extmark_id).build();

        let new_extmark_id = self
            .buffer
            .set_extmark(self.namespace_id, line, 0, &opts)
            .expect("couldn't move extmark");

        debug_assert_eq!(new_extmark_id, self.extmark_id);
    }

    /// Removes the handle from the buffer.
    pub(super) fn remove(mut self) {
        self.buffer
            .del_extmark(self.namespace_id, self.extmark_id)
            .expect("couldn't delete extmark");
    }

    /// Returns the line and options to give to [`api::Buffer::set_extmark`] to
    /// position the peer handle above or below the cursor at the given byte
    /// offset (the column is always zero).
    fn extmark_params(
        buffer: api::Buffer,
        cursor_offset: ByteOffset,
        peer_handle: &PeerHandle,
        hl_group_id: u32,
    ) -> (usize, api::opts::SetExtmarkOptsBuilder) {
        let cursor_point = buffer.point_of_byte(cursor_offset);

        let num_rows = buffer.num_rows();

        let line_idx =
            // If the cursor is not on the first line, place the handle on the
            // previous line so that it appears above the cursor.
            if cursor_point.newline_offset > 0 {
                cursor_point.newline_offset - 1
            }
            // Otherwise, try to place it on the next line so that it appears
            // below the cursor.
            else if num_rows > 1 {
                cursor_point.newline_offset + 1
            }
            // If the buffer has a single line, we'll use the virt_lines
            // approach to display the handle on a virtual line below the
            // cursor.
            else {
                0
            };

        let use_virt_lines = num_rows == 1;

        // FIXME: using the cursor's offset as the target column could result
        // in the handle being vertically misaligned if the cursor line
        // contains multi-byte characters.
        //
        // FIXME: this also doesn't handle tabs correctly. For those, we'd have
        // to count the number of tabs in the cursor line up to the cursor's
        // offset and multiply that by the value of the 'tabstop' option.
        //
        // FIXME: this also doesn't handle soft wraps correctly. I'm not sure
        // what to do about those.
        let target_col = cursor_point.byte_offset;

        let mut opts_builder = api::opts::SetExtmarkOpts::builder();

        let chunk = (format!(" {peer_handle} "), hl_group_id);

        if use_virt_lines {
            // When setting virt_lines, we have to add some padding for the
            // handle to align with the cursor's column.
            let padding = " ".repeat(target_col);
            let normal_group_id = NORMAL_HL_GROUP_ID.with(|id| **id);
            opts_builder.virt_lines([[(padding, normal_group_id), chunk]]);
        } else {
            opts_builder
                .virt_text([chunk])
                .virt_text_win_col(target_col as u32)
                .virt_text_pos(api::types::ExtmarkVirtTextPosition::Overlay);
        }

        (line_idx, opts_builder)
    }
}

impl PeerHighlightGroup for PeerHandleHighlightGroup {
    const NAME_PREFIX: &str = "NomadCollabPeerHandle";

    fn set_hl_opts() -> api::opts::SetHighlightOpts {
        api::opts::SetHighlightOpts::builder().link("PmenuSel").build()
    }

    fn with_group_ids<R>(fun: impl FnOnce(&[Cell<u32>]) -> R) -> R {
        Self::GROUP_IDS.with(|ids| fun(ids.as_array_of_cells().as_slice()))
    }
}
