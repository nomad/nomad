use nvimx_common::{ByteOffset, Point};

use crate::buffer_ctx::BufferCtx;
use crate::buffer_id::BufferId;
use crate::neovim_ctx::NeovimCtx;
use crate::pane_id::PaneId;

/// TODO: docs.
pub struct PaneCtx<'ctx> {
    pane_id: PaneId,
    neovim_ctx: NeovimCtx<'ctx>,
}

impl<'ctx> PaneCtx<'ctx> {
    /// Returns the [`PaneCtx`] of the currently focused pane.
    pub fn current(neovim_ctx: NeovimCtx<'ctx>) -> Self {
        Self { pane_id: PaneId::current(), neovim_ctx }
    }

    /// Returns the [`ByteOffset`] of the cursor in this pane, or `None` if the
    /// cursor is currently in another pane.
    pub fn cursor(&self) -> Option<ByteOffset> {
        self.is_focused().then(|| self.last_cursor())
    }

    /// Returns the [`BufferCtx`] of the buffer housed in this pane.
    pub fn housing(&self) -> BufferCtx<'_> {
        let win = self.pane_id.as_nvim();
        let id = win.get_buf().map(BufferId::new).expect("PaneId is valid");
        self.neovim_ctx.reborrow().into_buffer(id).expect("BufferId is valid")
    }

    /// TODO: docs.
    pub fn last_cursor(&self) -> ByteOffset {
        let win = self.pane_id.as_nvim();
        let (row, col) = win.get_cursor().expect("PaneId is valid");
        let point = Point { line_idx: row - 1, byte_offset: col.into() };
        self.housing().byte_offset_of_point(point)
    }

    pub(crate) fn new(pane_id: PaneId, ctx: NeovimCtx<'ctx>) -> Self {
        Self { pane_id, neovim_ctx: ctx }
    }

    fn is_focused(&self) -> bool {
        self.pane_id == PaneId::current()
    }
}
