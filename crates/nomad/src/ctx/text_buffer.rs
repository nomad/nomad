use alloc::borrow::Cow;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::{Bound, Deref, Range, RangeBounds};

use nvim_oxi::api::{self, types};

use crate::ctx::{BufferCtx, TextFileCtx};
use crate::neovim::BufferId;
use crate::{ByteOffset, Text};

/// TODO: docs.
#[derive(Clone)]
pub struct TextBufferCtx<'ctx> {
    buffer_ctx: BufferCtx<'ctx>,
}

/// The 2D equivalent of a [`ByteOffset`].
#[derive(Copy, Clone, PartialEq)]
struct Point {
    /// The index of the line in the buffer.
    pub(super) line_idx: usize,

    /// The byte offset in the line.
    pub(super) byte_offset: ByteOffset,
}

impl<'ctx> TextBufferCtx<'ctx> {
    /// Returns the text in the given byte range.
    ///
    /// # Panics
    ///
    /// Panics if the byte range is out of bounds.
    pub fn get_text<R>(&self, byte_range: R) -> Text
    where
        R: RangeBounds<ByteOffset>,
    {
        let point_range = self.point_range_of_byte_range(byte_range);
        self.get_text_in_point_range(point_range)
    }

    /// Consumes `self`, returning a [`TextFileCtx`] if the buffer is saved on
    /// disk, or `None` otherwise.
    pub fn into_text_file(self) -> Option<TextFileCtx<'ctx>> {
        TextFileCtx::from_text_buffer(self)
    }

    pub(crate) fn new(ctx: BufferCtx<'ctx>) -> Option<Self> {
        let nvim_buf = ctx.buffer_id().as_nvim();

        let opts =
            api::opts::OptionOpts::builder().buffer(nvim_buf.clone()).build();

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
    fn get_text_in_point_range(&self, point_range: Range<Point>) -> Text {
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

    /// Converts the given [`ByteOffset`] to the corresponding [`Point`] in the
    /// buffer.
    #[track_caller]
    fn point_of_byte_offset(&self, byte_offset: ByteOffset) -> Point {
        let nvim_buf = self.buffer_id().as_nvim();

        let line_idx = nvim_buf
            .call(move |_| {
                api::call_function::<_, usize>("byte2line", (byte_offset,))
                    .expect("args are valid")
            })
            .expect("todo");

        let line_byte_offset = nvim_buf.get_offset(line_idx).expect("todo");

        Point { line_idx, byte_offset: byte_offset - line_byte_offset }
    }

    /// Returns the [`Point`] at the end of the buffer.
    fn point_of_eof(&self) -> Point {
        fn point_of_eof(buffer: &BufferId) -> Result<Point, api::Error> {
            let nvim_buf = buffer.as_nvim();

            let num_lines = nvim_buf.line_count()?;

            if num_lines == 0 {
                return Ok(Point::zero());
            }

            let last_line_len = nvim_buf.get_offset(num_lines)?
            // `get_offset(line_count)` seems to always include the trailing
            // newline, even when `eol` is turned off.
            //
            // TODO: shouldn't we only correct this is `eol` is turned off?
            - 1
            - nvim_buf.get_offset(num_lines - 1)?;

            Ok(Point {
                line_idx: num_lines - 1,
                byte_offset: ByteOffset::new(last_line_len),
            })
        }

        let buffer_id = self.buffer_id();

        match point_of_eof(&buffer_id) {
            Ok(point) => point,
            Err(_) => panic!("{buffer_id:?} has been deleted"),
        }
    }

    /// Converts the given byte range into the corresponding point range in the
    /// buffer.
    #[track_caller]
    fn point_range_of_byte_range<R>(&self, byte_range: R) -> Range<Point>
    where
        R: RangeBounds<ByteOffset>,
    {
        let start = match byte_range.start_bound() {
            Bound::Excluded(&start) | Bound::Included(&start) => {
                self.point_of_byte_offset(start)
            },
            Bound::Unbounded => Point::zero(),
        };

        let end = match byte_range.end_bound() {
            Bound::Excluded(&end) => self.point_of_byte_offset(end),
            Bound::Included(&end) => self.point_of_byte_offset(end + 1),
            Bound::Unbounded => self.point_of_eof(),
        };

        start..end
    }
}

impl Point {
    pub(super) fn zero() -> Self {
        Self { line_idx: 0, byte_offset: ByteOffset::new(0) }
    }
}

impl<'ctx> Deref for TextBufferCtx<'ctx> {
    type Target = BufferCtx<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.buffer_ctx
    }
}
