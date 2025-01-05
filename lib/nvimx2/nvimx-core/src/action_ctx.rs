use core::ops::{Deref, DerefMut};

use smallvec::{SmallVec, smallvec};

use crate::backend::BackendExt;
use crate::{Backend, Name, NeovimCtx, notify};

/// TODO: docs.
pub struct ActionCtx<'a, B> {
    neovim_ctx: NeovimCtx<'a, B>,
    module_path: &'a ModulePath,
}

/// TODO: docs.
#[derive(Clone)]
pub(crate) struct ModulePath {
    names: SmallVec<[Name; 2]>,
}

impl<'a, B: Backend> ActionCtx<'a, B> {
    #[inline]
    pub(crate) fn emit_action_err<Err>(&mut self, action_name: Name, err: Err)
    where
        Err: notify::Error,
    {
        self.neovim_ctx.backend_mut().emit_action_err(
            &self.module_path,
            action_name,
            err,
        );
    }

    #[inline]
    pub(crate) fn module_path(&self) -> &ModulePath {
        &self.module_path
    }

    /// Constructs a new [`ActionCtx`].
    #[inline]
    pub(crate) fn new(
        neovim_ctx: NeovimCtx<'a, B>,
        module_path: &'a ModulePath,
    ) -> Self {
        Self { neovim_ctx, module_path }
    }
}

impl ModulePath {
    /// TODO: docs.
    #[inline]
    pub(crate) fn names(&self) -> impl ExactSizeIterator<Item = Name> + '_ {
        self.names.iter().copied()
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn new(base_module: Name) -> Self {
        Self { names: smallvec![base_module] }
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn push(&mut self, module_name: Name) {
        self.names.push(module_name);
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn pop(&mut self) {
        self.names.pop();
    }
}

impl<'a, B> Deref for ActionCtx<'a, B> {
    type Target = NeovimCtx<'a, B>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.neovim_ctx
    }
}

impl<B> DerefMut for ActionCtx<'_, B> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.neovim_ctx
    }
}
