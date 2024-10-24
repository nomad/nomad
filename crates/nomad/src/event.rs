use crate::autocmd::AutoCommand;
use crate::ctx::NeovimCtx;

/// TODO: docs.
pub trait Event: Sized {
    /// TODO: docs.
    type Ctx<'a>;

    /// TODO: docs.
    fn register(self, ctx: Self::Ctx<'_>);
}

impl<A: AutoCommand> Event for A {
    type Ctx<'a> = NeovimCtx<'a>;

    fn register(self, ctx: Self::Ctx<'_>) {
        let neovim_ctx = ctx.to_static();
        ctx.with_autocmd_map(move |map| {
            map.register(self, neovim_ctx);
        });
    }
}
