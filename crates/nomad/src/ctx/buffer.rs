use core::ops::Deref;

use crate::buffer_id::BufferId;
use crate::ctx::{FileCtx, NeovimCtx, TextBufferCtx};

/// TODO: docs.
#[derive(Clone)]
pub struct BufferCtx<'ctx> {
    buffer_id: BufferId,
    neovim_ctx: NeovimCtx<'ctx>,
}

impl<'ctx> BufferCtx<'ctx> {
    /// Returns the [`BufferId`].
    pub fn buffer_id(&self) -> BufferId {
        self.buffer_id
    }

    /// Consumes `self`, returning a [`FileCtx`] if the buffer is saved on
    /// disk, or `None` otherwise.
    pub fn into_file(self) -> Option<FileCtx<'ctx>> {
        FileCtx::from_buffer(self)
    }

    /// Consumes `self`, returning a [`TextBufferCtx`] if the buffer's content
    /// is text, or `None` otherwise.
    pub fn into_text_buffer(self) -> Option<TextBufferCtx<'ctx>> {
        TextBufferCtx::from_buffer(self)
    }

    /// TODO: docs.
    pub fn name(&self) -> String {
        self.buffer_id()
            .as_nvim()
            .get_name()
            .expect("the buffer is valid")
            // FIXME(noib3): `get_name()` should return a String.
            .display()
            .to_string()
    }

    /// TODO: docs.
    pub fn reborrow(&self) -> BufferCtx<'_> {
        BufferCtx {
            buffer_id: self.buffer_id,
            neovim_ctx: self.neovim_ctx.reborrow(),
        }
    }

    pub(crate) fn from_neovim(
        buffer_id: BufferId,
        neovim_ctx: NeovimCtx<'ctx>,
    ) -> Option<Self> {
        buffer_id
            .as_nvim()
            .is_valid()
            .then_some(Self { buffer_id, neovim_ctx })
    }
}

impl<'ctx> Deref for BufferCtx<'ctx> {
    type Target = NeovimCtx<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.neovim_ctx
    }
}
