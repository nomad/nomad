use nvim_oxi::api::types;

use crate::ctx::BufferCtx;
use crate::neovim::BufferId;

/// TODO: docs.
pub struct TextFileCtx<'ctx> {
    ctx: BufferCtx<'ctx>,
}
