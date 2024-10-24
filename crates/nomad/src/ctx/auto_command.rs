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

    /// Re-borrows `self`, returning a new [`AutoCommandCtx`] with a shorter
    /// lifetime.
    pub fn as_ref(&self) -> AutoCommandCtx<'_> {
        AutoCommandCtx {
            args: self.args.clone(),
            event: self.event,
            neovim_ctx: self.neovim_ctx.clone(),
        }
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
