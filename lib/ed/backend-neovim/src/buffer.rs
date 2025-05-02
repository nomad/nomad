//! TODO: docs.

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::ops::Range;
use std::borrow::Cow;
use std::path::PathBuf;

use compact_str::CompactString;
use ed_core::ByteOffset;
use ed_core::backend::{AgentId, Buffer, Edit, Replacement};

use crate::Neovim;
use crate::events::{self, EventHandle};
use crate::oxi::{BufHandle, api, mlua};

/// TODO: docs.
#[derive(Copy, Clone)]
pub struct NeovimBuffer<'a> {
    callbacks: &'a events::Callbacks,
    id: BufferId,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BufferId(BufHandle);

#[derive(Debug, Copy, Clone, PartialEq)]
struct Point {
    /// The index of the line in the buffer.
    line_idx: usize,

    /// The byte offset in the line.
    byte_offset: ByteOffset,
}

impl<'a> NeovimBuffer<'a> {
    #[inline]
    pub(crate) fn current(callbacks: &'a events::Callbacks) -> Self {
        Self::new(BufferId::of_focused(), callbacks)
    }

    #[inline]
    pub(crate) fn get_name(self) -> PathBuf {
        self.inner().get_name().expect("buffer exists")
    }

    #[inline]
    pub(crate) fn is_focused(self) -> bool {
        api::Window::current().get_buf().expect("window is valid")
            == self.inner()
    }

    #[inline]
    pub(crate) fn new(
        id: BufferId,
        callbacks: &'a events::Callbacks,
    ) -> Self {
        debug_assert!(id.is_valid());
        Self { id, callbacks }
    }

    /// Converts the arguments given to the
    /// [`on_bytes`](api::opts::BufAttachOptsBuilder::on_bytes) callback into
    /// the corresponding [`Replacement`].
    #[inline]
    pub(crate) fn replacement_of_on_bytes(
        &self,
        args: api::opts::OnBytesArgs,
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

        debug_assert_eq!(buf, self.inner());

        let deleted_range =
            (start_offset).into()..(start_offset + old_end_len).into();

        let start =
            Point { line_idx: start_row, byte_offset: start_col.into() };

        let end = Point {
            line_idx: start_row + new_end_row,
            byte_offset: (start_col * (new_end_row == 0) as usize
                + new_end_col)
                .into(),
        };

        let inserted_text = if start == end {
            Default::default()
        } else {
            self.get_text_in_point_range(start..end)
        };

        Replacement::new(deleted_range, &*inserted_text)
    }

    #[inline]
    pub(crate) fn selection(&self) -> Option<Range<ByteOffset>> {
        let mode = api::get_mode().expect("couldn't get mode").mode;

        if !(mode.is_visual() || mode.is_visual_select()) {
            return None;
        }

        let (anchor_row, anchor_col) = self.inner().get_mark('<').ok()?;

        let (head_row, head_col) = self.inner().get_mark('>').ok()?;

        let anchor = self.byte_offset_of_point(Point {
            line_idx: anchor_row - 1,
            byte_offset: ByteOffset::new(anchor_col),
        });

        let head = self.byte_offset_of_point(Point {
            line_idx: head_row - 1,
            byte_offset: ByteOffset::new(head_col),
        });

        match anchor.cmp(&head) {
            Ordering::Less => Some(anchor..head),
            Ordering::Equal => None,
            Ordering::Greater => Some(head..anchor),
        }
    }

    /// Converts the given [`Point`] to the corresponding [`ByteOffset`] in the
    /// buffer.
    #[track_caller]
    fn byte_offset_of_point(self, point: Point) -> ByteOffset {
        let line_offset: ByteOffset = self
            .inner()
            .get_offset(point.line_idx)
            .expect("couldn't get line offset")
            .into();
        line_offset + point.byte_offset
    }

    /// Returns the text in the given point range.
    #[track_caller]
    fn get_text_in_point_range(
        &self,
        point_range: Range<Point>,
    ) -> CompactString {
        let lines = self
            .inner()
            .get_text(
                point_range.start.line_idx..point_range.end.line_idx,
                point_range.start.byte_offset.into(),
                point_range.end.byte_offset.into(),
                &Default::default(),
            )
            .expect("couldn't get text");

        let mut text = CompactString::default();

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

    /// Returns the [`Point`] at the end of the buffer.
    fn point_of_eof(self) -> Point {
        fn point_of_eof(buffer: NeovimBuffer) -> Result<Point, api::Error> {
            let nvim_buf = buffer.inner();

            let num_lines = nvim_buf.line_count()?;

            if num_lines == 0 {
                return Ok(Point::zero());
            }

            let last_line_len = nvim_buf.get_offset(num_lines)?
            // `get_offset(line_count)` seems to always include the trailing
            // newline, even when `eol` is turned off.
            //
            // TODO: shouldn't we only correct this if `eol` is turned off?
            - 1
            - nvim_buf.get_offset(num_lines - 1)?;

            Ok(Point {
                line_idx: num_lines - 1,
                byte_offset: ByteOffset::new(last_line_len),
            })
        }

        point_of_eof(self).expect("not deleted")
    }

    #[inline]
    fn inner(&self) -> api::Buffer {
        debug_assert!(self.id.is_valid());
        self.id.0.into()
    }
}

impl BufferId {
    /// TODO: docs.
    #[inline]
    pub fn handle(self) -> BufHandle {
        self.0
    }

    #[inline]
    pub(crate) fn is_valid(self) -> bool {
        api::Buffer::from(self).is_valid()
    }

    #[inline]
    pub(crate) fn of_focused() -> Self {
        Self::new(api::Buffer::current())
    }

    #[inline]
    pub(crate) fn new(inner: api::Buffer) -> Self {
        Self(inner.handle())
    }
}

impl Point {
    /// TODO: docs.
    fn zero() -> Self {
        Self { line_idx: 0, byte_offset: ByteOffset::new(0) }
    }
}

impl Buffer for NeovimBuffer<'_> {
    type Backend = Neovim;
    type EventHandle = EventHandle;
    type Id = BufferId;

    #[inline]
    fn byte_len(&self) -> ByteOffset {
        self.byte_offset_of_point(self.point_of_eof())
    }

    #[inline]
    fn id(&self) -> Self::Id {
        self.id
    }

    #[inline]
    fn name(&self) -> Cow<'_, str> {
        self.get_name().to_string_lossy().into_owned().into()
    }

    #[inline]
    fn on_edited<Fun>(&mut self, mut fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&NeovimBuffer<'_>, &Edit) + 'static,
    {
        self.callbacks.insert_callback_for(
            events::OnBytes(self.id()),
            move |(this, edit)| fun(this, edit),
        )
    }

    #[inline]
    fn on_removed<Fun>(&mut self, mut fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&NeovimBuffer<'_>, AgentId) + 'static,
    {
        self.callbacks.insert_callback_for(
            events::BufUnload(self.id()),
            move |(this, removed_by)| fun(this, removed_by),
        )
    }

    #[inline]
    fn on_saved<Fun>(&mut self, mut fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&NeovimBuffer<'_>, AgentId) + 'static,
    {
        self.callbacks.insert_callback_for(
            events::BufWritePost(self.id()),
            move |(this, saved_by)| fun(this, saved_by),
        )
    }
}

impl From<NeovimBuffer<'_>> for api::Buffer {
    #[inline]
    fn from(buf: NeovimBuffer) -> Self {
        buf.id().into()
    }
}

impl From<BufferId> for api::Buffer {
    #[inline]
    fn from(buf_id: BufferId) -> Self {
        buf_id.0.into()
    }
}

impl mlua::IntoLua for BufferId {
    #[inline]
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        self.handle().into_lua(lua)
    }
}

impl Hash for BufferId {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_i32(self.handle());
    }
}

impl nohash::IsEnabled for BufferId {}
