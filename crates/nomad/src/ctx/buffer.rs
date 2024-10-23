use crate::ctx::{FileCtx, NeovimCtx, TextBufferCtx};
use crate::neovim::BufferId;

/// TODO: docs.
#[derive(Clone)]
pub struct BufferCtx<'ctx> {
    buffer_id: BufferId,
    neovim_ctx: NeovimCtx<'ctx>,
}

impl<'ctx> BufferCtx<'ctx> {
    /// Returns the [`BufferId`].
    pub fn buffer_id(&self) -> BufferId {
        self.buffer_id.clone()
    }

    /// Consumes `self`, returning a [`FileCtx`] if the buffer is saved on
    /// disk, or `None` otherwise.
    pub fn into_file(self) -> Option<FileCtx<'ctx>> {
        FileCtx::new(self)
    }

    /// Consumes `self`, returning a [`TextBufferCtx`] if the buffer's content
    /// is text, or `None` otherwise.
    pub fn into_text_buffer(self) -> Option<TextBufferCtx<'ctx>> {
        TextBufferCtx::new(self)
    }

    pub fn name(&self) -> String {
        self.buffer_id()
            .as_nvim()
            .get_name()
            .expect("the buffer is valid")
            // FIXME(noib3): `get_name()` should return a String.
            .display()
            .to_string()
    }

    pub(crate) fn new(
        buffer_id: BufferId,
        neovim_ctx: NeovimCtx<'ctx>,
    ) -> Option<Self> {
        buffer_id
            .as_nvim()
            .is_valid()
            .then_some(Self { buffer_id, neovim_ctx })
    }
}
