use nvim_oxi::api::types;

use crate::actor_map::ActorMap;
use crate::ctx::BufferCtx;

/// TODO: docs.
pub struct FileCtx<'ctx> {
    buffer_ctx: BufferCtx<'ctx>,
}
