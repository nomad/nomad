use crate::file_ctx::FileCtx;
use crate::text_buffer_ctx::TextBufferCtx;

/// TODO: docs.
#[derive(Clone)]
pub struct TextFileCtx<'ctx> {
    file_ctx: FileCtx<'ctx>,
}

impl<'ctx> TextFileCtx<'ctx> {
    /// TODO: docs.
    pub fn as_file(&self) -> &FileCtx<'ctx> {
        &self.file_ctx
    }

    /// TODO: docs.
    pub fn as_text_buffer(&self) -> &TextBufferCtx<'ctx> {
        TextBufferCtx::new_ref_unchecked(&self.file_ctx)
    }

    pub(crate) fn from_file(file_ctx: FileCtx<'ctx>) -> Option<Self> {
        TextBufferCtx::from_buffer((*file_ctx).clone())
            .is_some()
            .then_some(Self { file_ctx })
    }

    pub(crate) fn from_text_buffer(
        text_buffer_ctx: TextBufferCtx<'ctx>,
    ) -> Option<Self> {
        FileCtx::from_buffer(text_buffer_ctx.into_buffer())
            .map(|file_ctx| Self { file_ctx })
    }
}
