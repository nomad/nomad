use core::ops::{Deref, Range, RangeBounds};

use nvim_oxi::api::{self, opts};
use nvimx_action::{Action, IntoModuleName};
use nvimx_common::{ByteOffset, Point, Replacement, Text};

use crate::autocmd::ShouldDetach;
use crate::buf_attach::BufAttachArgs;
use crate::buffer_ctx::BufferCtx;
use crate::text_file_ctx::TextFileCtx;
use crate::ActorId;

/// TODO: docs.
#[derive(Clone)]
#[repr(transparent)]
pub struct TextBufferCtx<'ctx> {
    buffer_ctx: BufferCtx<'ctx>,
}

impl<'ctx> TextBufferCtx<'ctx> {
    /// TODO: docs.
    pub fn attach<M, A>(&self, action: A)
    where
        M: IntoModuleName,
        A: for<'a> Action<
            M,
            Ctx<'a> = BufferCtx<'a>,
            Args: From<BufAttachArgs>,
            Return: Into<ShouldDetach>,
        >,
    {
        self.with_buf_attach_map(|m| {
            let neovim_ctx = self.to_static();
            m.register(self.buffer_id(), action, neovim_ctx);
        });
    }

    /// Returns the text in the given byte range.
    ///
    /// # Panics
    ///
    /// Panics if the byte range is out of bounds.
    pub fn get_text<R>(&self, byte_range: R) -> Text
    where
        R: RangeBounds<ByteOffset>,
    {
        let point_range = self.point_range_of_byte_range(&byte_range);
        self.get_text_in_point_range(point_range)
    }

    /// Consumes `self`, returning the underlying [`BufferCtx`].
    pub fn into_buffer(self) -> BufferCtx<'ctx> {
        self.buffer_ctx
    }

    /// Consumes `self`, returning a [`TextFileCtx`] if the buffer is saved on
    /// disk, or `None` otherwise.
    pub fn into_text_file(self) -> Option<TextFileCtx<'ctx>> {
        TextFileCtx::from_text_buffer(self)
    }

    /// TODO: docs.
    pub fn reborrow(&self) -> TextBufferCtx<'_> {
        TextBufferCtx { buffer_ctx: self.buffer_ctx.reborrow() }
    }

    /// Replaces the text in the given byte range with the given text.
    ///
    /// # Panics
    ///
    /// Panics if the byte range is out of bounds.
    pub fn replace_text<R>(
        &self,
        delete_range: R,
        insert_text: &Text,
        actor_id: ActorId,
    ) where
        R: RangeBounds<ByteOffset>,
    {
        let point_range = self.point_range_of_byte_range(&delete_range);
        self.replace_text_in_point_range(point_range, insert_text.as_str());
        self.with_actor_map(|map| {
            map.edited_buffer(self.buffer_id(), actor_id);
        });
    }

    pub(crate) fn from_buffer(ctx: BufferCtx<'ctx>) -> Option<Self> {
        let nvim_buf = ctx.buffer_id().as_nvim();

        let opts =
            opts::OptionOpts::builder().buffer(nvim_buf.clone()).build();

        let is_text_file = nvim_buf.is_loaded()
            // Checks that the buftype is empty. This filters out help and
            // terminal buffers, the quickfix list, etc.
            && api::get_option_value::<String>("buftype", &opts)
                    .ok()
                    .map(|buftype| buftype.is_empty())
                    .unwrap_or(false)
            // Checks that the buffer contents are UTF-8 encoded, which filters
            // out buffers containing binary data.
            && api::get_option_value::<String>("fileencoding", &opts)
                    .ok()
                    .map(|mut encoding| {
                        encoding.make_ascii_lowercase();
                        encoding == "utf-8"
                    })
                    .unwrap_or(false);

        is_text_file.then_some(Self { buffer_ctx: ctx })
    }

    /// Returns the text in the given point range.
    #[track_caller]
    pub(crate) fn get_text_in_point_range(
        &self,
        point_range: Range<Point>,
    ) -> Text {
        let lines = match self.buffer_id().as_nvim().get_text(
            point_range.start.line_idx..point_range.end.line_idx,
            point_range.start.byte_offset.into(),
            point_range.end.byte_offset.into(),
            &Default::default(),
        ) {
            Ok(lines) => lines,
            Err(err) => panic!("{err}"),
        };

        let mut text = Text::new();

        let num_lines = lines.len();

        for (idx, line) in lines.enumerate() {
            let line = line.to_str().expect("line is not UTF-8");
            text.push_str(line);
            let is_last = idx + 1 == num_lines;
            if !is_last {
                text.push('\n');
            }
        }

        text
    }

    pub(crate) fn new_unchecked(buffer_ctx: BufferCtx<'ctx>) -> Self {
        debug_assert!(Self::from_buffer(buffer_ctx.clone()).is_some());
        Self { buffer_ctx }
    }

    pub(crate) fn new_ref_unchecked<'a>(ctx: &'a BufferCtx<'ctx>) -> &'a Self {
        debug_assert!(Self::from_buffer(ctx.clone()).is_some());
        // SAFETY: `TextBufferCtx` is a `repr(transparent)` newtype over
        // `BufferCtx`.
        unsafe { &*(ctx as *const BufferCtx<'ctx> as *const Self) }
    }

    /// Converts the arguments given to the
    /// [`on_bytes`](nvim_oxi::api::opts::BufAttachOptsBuilder::on_bytes)
    /// callback into the corresponding [`Replacement`].
    pub(crate) fn replacement_of_on_bytes_args(
        &self,
        args: opts::OnBytesArgs,
    ) -> Replacement {
        let (
            _bytes,
            buf,
            _changedtick,
            start_row,
            start_col,
            start_offset,
            _old_end_row,
            _old_end_col,
            old_end_len,
            new_end_row,
            new_end_col,
            _new_end_len,
        ) = args;

        debug_assert_eq!(buf, self.buffer_id().as_nvim());

        let deleted_range =
            (start_offset).into()..(start_offset + old_end_len).into();

        let start =
            Point { line_idx: start_row, byte_offset: start_offset.into() };

        let end = Point {
            line_idx: start_row + new_end_row,
            byte_offset: (start_col * (new_end_row == 0) as usize
                + new_end_col)
                .into(),
        };

        let inserted_text = if start == end {
            Text::new()
        } else {
            self.get_text_in_point_range(start..end)
        };

        Replacement::new(deleted_range, inserted_text)
    }

    /// Replaces the text in the given point range with the given text.
    #[track_caller]
    fn replace_text_in_point_range(
        &self,
        delete_range: Range<Point>,
        insert_text: &str,
    ) {
        // If the text has a trailing newline, Neovim expects an additional
        // empty line to be included.
        let lines = insert_text
            .lines()
            .chain(insert_text.ends_with('\n').then_some(""));

        self.buffer_id()
            .as_nvim()
            .set_text(
                delete_range.start.line_idx..delete_range.end.line_idx,
                delete_range.start.byte_offset.into(),
                delete_range.end.byte_offset.into(),
                lines,
            )
            .expect("replacing text failed");
    }
}

impl<'ctx> Deref for TextBufferCtx<'ctx> {
    type Target = BufferCtx<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.buffer_ctx
    }
}
