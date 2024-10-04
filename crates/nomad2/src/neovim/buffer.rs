use alloc::borrow::Cow;
use core::cmp::Ordering;
use core::fmt;
use core::ops::{Bound, Range, RangeBounds};

use collab_fs::{AbsUtf8Path, AbsUtf8PathBuf};
use futures_util::stream::{pending, Pending};
use nvim_oxi::api::{self, Buffer as NvimBuffer};

use super::Neovim;
use crate::{ByteOffset, Context, Edit, Emitter, Event, Subscription, Text};

/// TODO: docs.
pub struct Buffer {
    id: BufferId,
}

/// TODO: docs.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BufferId {
    inner: NvimBuffer,
}

/// TODO: docs.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct EditEvent {
    id: BufferId,
}

/// The 2D equivalent of a `ByteOffset`.
struct Point {
    /// The index of the line in the buffer.
    line_idx: usize,

    /// The byte offset in the line.
    byte_offset: ByteOffset,
}

impl Buffer {
    pub(super) fn new(id: BufferId) -> Self {
        Self { id }
    }

    fn as_nvim(&self) -> &NvimBuffer {
        &self.id.inner
    }

    fn as_nvim_mut(&mut self) -> &mut NvimBuffer {
        &mut self.id.inner
    }

    #[track_caller]
    fn byte_offset_of_point(&self, point: Point) -> ByteOffset {
        point.byte_offset
            + self.as_nvim().get_offset(point.line_idx).expect("todo")
    }

    /// # Panics
    ///
    /// Panics if the point range is out of bounds or if the buffer has been
    /// deleted or unloaded.
    #[track_caller]
    fn get_text_in_point_range(&self, point_range: Range<Point>) -> Text {
        let lines = match self.as_nvim().get_text(
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

    #[track_caller]
    fn point_of_byte_offset(&self, byte_offset: ByteOffset) -> Point {
        let buf = self.as_nvim();

        let line_idx = buf
            .call(move |_| {
                api::call_function::<_, usize>("byte2line", (byte_offset,))
                    .expect("args are valid")
            })
            .expect("todo");

        let line_byte_offset = buf.get_offset(line_idx).expect("todo");

        Point { line_idx, byte_offset: byte_offset - line_byte_offset }
    }

    fn point_of_eof(&self) -> Point {
        fn point_of_eof(buffer: &Buffer) -> Result<Point, api::Error> {
            let buf = buffer.as_nvim();

            let num_lines = buf.line_count()?;

            if num_lines == 0 {
                return Ok(Point::zero());
            }

            let last_line_len = buf.get_offset(num_lines)?
            // `get_offset(line_count)` seems to always include the trailing
            // newline, even when `eol` is turned off.
            //
            // TODO: shouldn't we only correct this is `eol` is turned off?
            - 1
            - buf.get_offset(num_lines - 1)?;

            Ok(Point {
                line_idx: num_lines - 1,
                byte_offset: ByteOffset::new(last_line_len),
            })
        }

        match point_of_eof(self) {
            Ok(point) => point,
            Err(_) => panic!("{self:?} has been deleted"),
        }
    }

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

    /// # Panics.
    ///
    /// Panics if the point range is out of bounds or if the buffer has been
    /// deleted or unloaded.
    #[track_caller]
    fn replace_text_in_point_range(
        &mut self,
        point_range: Range<Point>,
        replacement: &str,
    ) {
        // If the text has a trailing newline, Neovim expects an additional
        // empty line to be included.
        let lines = replacement
            .lines()
            .chain(replacement.ends_with('\n').then_some(""));

        if let Err(err) = self.as_nvim_mut().set_text(
            point_range.start.line_idx..point_range.end.line_idx,
            point_range.start.byte_offset.into(),
            point_range.end.byte_offset.into(),
            lines,
        ) {
            panic!("{err}");
        }
    }
}

impl crate::Buffer<Neovim> for Buffer {
    type EditStream = Subscription<EditEvent, Neovim>;
    type Id = BufferId;

    fn edit_stream(&mut self, ctx: &Context<Neovim>) -> Self::EditStream {
        ctx.subscribe(EditEvent { id: self.id.clone() })
    }

    fn get_text<R>(&self, byte_range: R) -> Text
    where
        R: RangeBounds<ByteOffset>,
    {
        let point_range = self.point_range_of_byte_range(byte_range);
        self.get_text_in_point_range(point_range)
    }

    fn id(&self) -> Self::Id {
        self.id.clone()
    }

    #[track_caller]
    fn path(&self) -> Option<Cow<'_, AbsUtf8Path>> {
        self.id.is_of_text_buffer().then(|| {
            let Ok(path) = self.as_nvim().get_name() else {
                panic!("{self:?} has been deleted");
            };
            Cow::Owned(
                AbsUtf8PathBuf::from_path_buf(path)
                    .expect("path is absolute and valid UTF-8"),
            )
        })
    }

    fn set_text<R, T>(&mut self, replaced_range: R, new_text: T)
    where
        R: RangeBounds<ByteOffset>,
        T: AsRef<str>,
    {
        let point_range = self.point_range_of_byte_range(replaced_range);
        self.replace_text_in_point_range(point_range, new_text.as_ref());
    }
}

impl fmt::Debug for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Buffer").field(&self.as_nvim().handle()).finish()
    }
}

impl BufferId {
    pub(super) fn is_of_text_buffer(&self) -> bool {
        let opts = api::opts::OptionOpts::builder()
            .buffer(self.inner.clone())
            .build();

        self.inner.is_loaded()
        // Checks that the buftype is empty. This filters out help and terminal
        // buffers, the quickfix list, etc.
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
                .unwrap_or(false)
    }

    pub(super) fn new(inner: NvimBuffer) -> Self {
        Self { inner }
    }
}

impl PartialOrd for BufferId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BufferId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.handle().cmp(&other.inner.handle())
    }
}

impl Event<Neovim> for EditEvent {
    type Payload = Edit;
    type SubscribeCtx = BufferId;

    fn subscribe(
        &mut self,
        emitter: Emitter<Self::Payload>,
        _: &Context<Neovim>,
    ) -> Self::SubscribeCtx {
        todo!()
    }

    fn unsubscribe(
        &mut self,
        sub_ctx: Self::SubscribeCtx,
        _: &Context<Neovim>,
    ) {
        todo!();
    }
}

impl Point {
    fn zero() -> Self {
        Self { line_idx: 0, byte_offset: ByteOffset::new(0) }
    }
}
