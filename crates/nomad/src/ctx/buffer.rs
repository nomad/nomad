use crate::ctx::NeovimCtx;
use crate::neovim::BufferId;

/// TODO: docs.
pub struct BufferCtx<'ctx> {
    buffer_id: BufferId,
    neovim_ctx: NeovimCtx<'ctx>,
}

impl<'ctx> BufferCtx<'ctx> {
    pub fn buffer_id(&self) -> BufferId {
        self.buffer_id.clone()
    }

    pub(crate) fn new(
        buffer_id: BufferId,
        neovim_ctx: NeovimCtx<'ctx>,
    ) -> Option<Self> {
        todo!();
    }
}
