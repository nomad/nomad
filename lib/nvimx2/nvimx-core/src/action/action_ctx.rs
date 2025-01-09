use core::ops::{Deref, DerefMut};

use crate::backend::Backend;
use crate::notify::{self, Name, NotificationId};
use crate::{NeovimCtx, Plugin};

/// TODO: docs.
pub struct ActionCtx<'a, P, B> {
    neovim_ctx: NeovimCtx<'a, P, B>,
    action_name: Name,
}

impl<'a, P, B> ActionCtx<'a, P, B>
where
    P: Plugin<B>,
    B: Backend,
{
    /// TODO: docs.
    #[inline]
    pub fn emit_info(&mut self, message: notify::Message) -> NotificationId {
        self.neovim_ctx.emit_info_inner(message, None)
    }

    #[inline]
    pub(crate) fn emit_err<Err>(&mut self, err: Err)
    where
        Err: notify::Error<B>,
    {
        self.neovim_ctx.emit_err(Some(self.action_name), err);
    }

    #[inline]
    pub(crate) fn new(
        neovim_ctx: NeovimCtx<'a, P, B>,
        action_name: Name,
    ) -> Self {
        Self { neovim_ctx, action_name }
    }
}

impl<'a, P, B> Deref for ActionCtx<'a, P, B> {
    type Target = NeovimCtx<'a, P, B>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.neovim_ctx
    }
}

impl<P, B> DerefMut for ActionCtx<'_, P, B> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.neovim_ctx
    }
}
