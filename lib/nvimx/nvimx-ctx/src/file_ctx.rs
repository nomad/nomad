use core::ops::Deref;

use nvimx_fs::{AbsPath, AbsPathBuf};

use crate::buffer_ctx::BufferCtx;
use crate::text_file_ctx::TextFileCtx;

/// TODO: docs.
#[derive(Clone)]
pub struct FileCtx<'ctx> {
    pub(super) file_path: AbsPathBuf,
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

    /// Returns the absolute path to the file.
    pub fn path(&self) -> &AbsPath {
        &self.file_path
    }

    pub(crate) fn from_buffer(buffer_ctx: BufferCtx<'ctx>) -> Option<Self> {
        let buffer_name = buffer_ctx.name();
        let file_path = buffer_name.parse::<AbsPathBuf>().ok()?;
        std::fs::metadata(&file_path)
            .is_ok()
            .then_some(Self { file_path, buffer_ctx })
    }
}

impl<'ctx> Deref for FileCtx<'ctx> {
    type Target = BufferCtx<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.buffer_ctx
    }
}
