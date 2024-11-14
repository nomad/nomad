use core::ops::{Bound, Deref, Range, RangeBounds};

use nvim_oxi::api;
use nvimx_common::{ByteOffset, Point};
use nvimx_diagnostics::HighlightGroup;

use crate::buffer_id::BufferId;
use crate::decoration_provider::Selection;
use crate::file_ctx::FileCtx;
use crate::neovim_ctx::NeovimCtx;
use crate::text_buffer_ctx::TextBufferCtx;

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

    /// Converts the given [`Point`] to the corresponding [`ByteOffset`] in the
    /// buffer.
    #[track_caller]
    pub fn byte_offset_of_point(&self, point: Point) -> ByteOffset {
        let line_offset: ByteOffset = self
            .buffer_id()
            .as_nvim()
            .get_offset(point.line_idx)
            .expect("couldn't get line offset")
            .into();
        line_offset + point.byte_offset
    }

    /// Returns the byte length of the buffer.
    pub fn byte_len(&self) -> ByteOffset {
        self.byte_offset_of_point(self.point_of_eof())
    }

    /// TODO: docs.
    pub fn create_selection(
        &self,
        byte_range: Range<ByteOffset>,
        hl_group: HighlightGroup,
    ) -> Selection {
        self.with_decoration_provider(|decoration_provider| {
            decoration_provider.create_selection(
                self.buffer_id(),
                byte_range,
                hl_group,
            )
        })
    }

    /// Returns the [`BufferCtx`] of the current buffer.
    pub fn current(neovim_ctx: NeovimCtx<'ctx>) -> Self {
        Self { buffer_id: BufferId::current(), neovim_ctx }
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

    /// Converts the given [`ByteOffset`] to the corresponding [`Point`] in the
    /// buffer.
    #[track_caller]
    pub fn point_of_byte_offset(&self, byte_offset: ByteOffset) -> Point {
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
    pub fn point_of_eof(&self) -> Point {
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
    pub fn point_range_of_byte_range<R>(&self, byte_range: &R) -> Range<Point>
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
