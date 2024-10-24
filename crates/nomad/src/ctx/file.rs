use core::ops::Deref;
use std::path::PathBuf;

use crate::ctx::{BufferCtx, TextFileCtx};

/// TODO: docs.
#[derive(Clone)]
pub struct FileCtx<'ctx> {
    pub(super) file_path: PathBuf,
    pub(super) buffer_ctx: BufferCtx<'ctx>,
}

impl<'ctx> FileCtx<'ctx> {
    /// Consumes `self`, returning the underlying [`BufferCtx`].
    pub fn into_buffer(self) -> BufferCtx<'ctx> {
        self.buffer_ctx
    }

    /// Consumes `self`, returning a [`TextFileCtx`] if the file's content
    /// is text, or `None` otherwise.
    pub fn into_text_file(self) -> Option<TextFileCtx<'ctx>> {
        TextFileCtx::from_file(self)
    }

    pub(crate) fn from_buffer(buffer_ctx: BufferCtx<'ctx>) -> Option<Self> {
        let buffer_name = buffer_ctx.name();
        let file_path = buffer_name.parse::<PathBuf>().ok()?;
        file_path.exists().then_some(Self { file_path, buffer_ctx })
    }
}

impl<'ctx> Deref for FileCtx<'ctx> {
    type Target = BufferCtx<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.buffer_ctx
    }
}
