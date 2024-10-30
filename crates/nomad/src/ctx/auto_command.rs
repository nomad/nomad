use core::ops::Deref;

use nvim_oxi::api::types;

use crate::actor_map::ActorMap;
use crate::autocmd::AutoCommandEvent;
use crate::ctx::NeovimCtx;

/// TODO: docs.
pub struct AutoCommandCtx<'ctx> {
    args: types::AutocmdCallbackArgs,
    event: AutoCommandEvent,
    neovim_ctx: NeovimCtx<'ctx>,
}

impl<'ctx> AutoCommandCtx<'ctx> {
    /// Returns a shared reference to the autocmd's args.
    pub fn args(&self) -> &types::AutocmdCallbackArgs {
        &self.args
    }

    /// TODO: docs.
    pub fn as_neovim(&self) -> &NeovimCtx<'_> {
        &self.neovim_ctx
    }

    /// Returns the event that triggered the autocmd.
    pub fn event(&self) -> AutoCommandEvent {
        self.event
    }

    /// Consumes `self` and returns the arguments passed to the autocmd.
    pub fn into_args(self) -> types::AutocmdCallbackArgs {
        self.args
    }

    /// Calls the given clousure with an exlusive reference to the
    /// [`ActorMap`].
    pub(crate) fn with_actor_map<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(&mut ActorMap) -> R,
    {
        self.neovim_ctx.with_actor_map(fun)
    }

    pub(crate) fn new(
        args: types::AutocmdCallbackArgs,
        event: AutoCommandEvent,
        neovim_ctx: NeovimCtx<'ctx>,
    ) -> Self {
        Self { args, event, neovim_ctx }
    }
}

impl<'ctx> Deref for AutoCommandCtx<'ctx> {
    type Target = NeovimCtx<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.neovim_ctx
    }
}
