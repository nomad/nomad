use alloc::borrow::Cow;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::{Bound, Range, RangeBounds};

use collab_fs::{AbsUtf8Path, AbsUtf8PathBuf};
use nvim_oxi::api::{self, Buffer as NvimBuffer};

use super::events::{CursorEvent, EditEvent};
use super::{Neovim, Offset};
use crate::{ActorId, ByteOffset, Context, Shared, Subscription, Text};

/// TODO: docs.
pub struct Buffer {
    id: BufferId,

    /// TODO: docs.
    next_cursor_moved_by: Shared<Option<ActorId>>,

    /// TODO: docs.
    next_edit_made_by: Shared<Option<ActorId>>,
}

/// TODO: docs.
#[derive(Clone, PartialEq, Eq)]
pub struct BufferId {
    inner: NvimBuffer,
}

impl Buffer {
    /// TODO: docs.
    pub fn cursor_stream(
        &mut self,
        ctx: &Context<Neovim>,
    ) -> Subscription<CursorEvent, Neovim> {
        ctx.subscribe(CursorEvent {
            id: self.id.clone(),
            next_cursor_moved_by: self.next_cursor_moved_by.clone(),
        })
    }

    /// TODO: docs.
    pub fn edit_stream<T: Offset + Clone>(
        &mut self,
        ctx: &Context<Neovim>,
    ) -> Subscription<EditEvent<T>, Neovim> {
        ctx.subscribe(EditEvent::new(
            self.id.clone(),
            self.next_edit_made_by.clone(),
        ))
    }

    pub(super) fn new(
        id: BufferId,
        next_cursor_moved_by: Shared<Option<ActorId>>,
        next_edit_made_by: Shared<Option<ActorId>>,
    ) -> Self {
        Self { id, next_cursor_moved_by, next_edit_made_by }
    }
}

impl crate::Buffer<Neovim> for Buffer {
    type Id = BufferId;

    fn get_text<R>(&self, byte_range: R) -> Text
    where
        R: RangeBounds<ByteOffset>,
    {
        todo!();
    }

    fn id(&self) -> Self::Id {
        self.id.clone()
    }

    #[track_caller]
    fn path(&self) -> Option<Cow<'_, AbsUtf8Path>> {
        self.id.is_of_text_buffer().then(|| {
            Cow::Owned(
                self.id.path().expect("checked that id is of text buffer"),
            )
        })
    }

    fn set_text<R, T>(
        &mut self,
        replaced_range: R,
        new_text: T,
        actor_id: ActorId,
    ) where
        R: RangeBounds<ByteOffset>,
        T: AsRef<str>,
    {
        let point_range = self.id.point_range_of_byte_range(replaced_range);
        self.id.replace_text_in_point_range(point_range, new_text.as_ref());
        self.next_edit_made_by.set(Some(actor_id));
    }
}

impl fmt::Debug for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Buffer").field(&self.id.handle()).finish()
    }
}

impl BufferId {
    /// Returns the [`BufferId`] of the buffer at the given path, if there is
    /// one.
    pub fn from_path(path: &AbsUtf8Path) -> Option<Self> {
        api::call_function::<_, i32>("bufnr", (path.to_string(),))
            .ok()
            .and_then(|id| {
                (id != -1).then_some(Self { inner: NvimBuffer::from(id) })
            })
    }

    /// TODO: docs.
    pub fn path(&self) -> Option<AbsUtf8PathBuf> {
        self.inner
            .get_name()
            .ok()
            .and_then(|name| AbsUtf8PathBuf::from_path_buf(name).ok())
    }

    pub(crate) fn current() -> Self {
        Self::new(NvimBuffer::current())
    }

    pub(crate) fn as_nvim(&self) -> NvimBuffer {
        self.inner.clone()
    }

    /// # Panics
    ///
    /// Panics if the point range is out of bounds or if the buffer has been
    /// deleted or unloaded.
    #[track_caller]
    pub(super) fn get_text_in_point_range(
        &self,
        point_range: Range<Point>,
    ) -> Text {
        let lines = match self.inner.get_text(
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

    pub(super) fn is_of_text_buffer(&self) -> bool {
        true
    }

    pub(crate) fn new(inner: NvimBuffer) -> Self {
        Self { inner }
    }

    #[track_caller]
    fn byte_offset_of_point(&self, point: Point) -> ByteOffset {
        point.byte_offset
            + self.inner.get_offset(point.line_idx).expect("todo")
    }

    fn handle(&self) -> i32 {
        self.inner.handle()
    }

    #[track_caller]
    fn point_of_byte_offset(&self, byte_offset: ByteOffset) -> Point {
        let buf = &self.inner;

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
        fn point_of_eof(buffer: &BufferId) -> Result<Point, api::Error> {
            let buf = &buffer.inner;

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

        if let Err(err) = self.inner.set_text(
            point_range.start.line_idx..point_range.end.line_idx,
            point_range.start.byte_offset.into(),
            point_range.end.byte_offset.into(),
            lines,
        ) {
            panic!("{err}");
        }
    }
}

impl fmt::Debug for BufferId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("BufferId").field(&self.inner.handle()).finish()
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

impl Hash for BufferId {
    #[inline]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        hasher.write_i32(self.inner.handle());
    }
}

impl nohash::IsEnabled for BufferId {}
