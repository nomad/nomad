use core::cell::Cell;
use core::ops::Range;

use collab_types::{Peer, PeerHandle, PeerId};
use editor::ByteOffset;
use neovim::buffer::{BufferExt, Point};
use neovim::oxi::api;

use crate::editors::neovim::PeerHighlightGroup;

/// A remote peer's handle in a buffer, displayed either directly above or
/// below their [`cursor`](crate::editors::neovim::PeerCursor).
pub struct NeovimPeerHandle {
    /// The buffer the cursor is in.
    buffer: api::Buffer,

    /// The ID of the extmark used to display the handle.
    extmark_id: u32,

    /// The ID of the namespace the [`extmark_id`](Self::extmark_id) belongs
    /// to.
    namespace_id: u32,

    /// The remote peer's handle.
    peer_handle: PeerHandle,

    /// The remote peer's ID.
    peer_id: PeerId,
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
        _peer: Peer,
        _buffer: api::Buffer,
        _cursor_offset: ByteOffset,
        _namespace_id: u32,
    ) -> Self {
        todo!();
    }

    /// Moves the handle to keep it in sync with the new cursor offset.
    pub(super) fn r#move(&mut self, _new_cursor_offset: ByteOffset) {
        todo!();
    }

    /// Removes the handle from the buffer.
    pub(super) fn remove(mut self) {
        self.buffer
            .del_extmark(self.namespace_id, self.extmark_id)
            .expect("couldn't delete extmark");
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
