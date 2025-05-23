//! TODO: docs.

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::ops::Range;
use std::borrow::Cow;
use std::path::PathBuf;

use compact_str::CompactString;
use ed::backend::{AgentId, Buffer, Edit, Replacement};
use ed::fs::AbsPath;
use ed::{ByteOffset, Shared};

use crate::Neovim;
use crate::cursor::NeovimCursor;
use crate::events::{self, EventHandle, Events};
use crate::oxi::{BufHandle, api, mlua};

/// TODO: docs.
#[derive(Copy, Clone)]
pub struct NeovimBuffer<'a> {
    events: &'a Shared<Events>,
    id: BufferId,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BufferId(BufHandle);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct Point {
    /// The index of the line in the buffer.
    pub(crate) line_idx: usize,

    /// The byte offset in the line.
    pub(crate) byte_offset: ByteOffset,
}

impl<'a> NeovimBuffer<'a> {
    /// Converts the given [`Point`] to the corresponding [`ByteOffset`] in the
    /// buffer.
    #[track_caller]
    #[inline]
    pub(crate) fn byte_offset_of_point(self, point: Point) -> ByteOffset {
        let line_offset: ByteOffset = self
            .inner()
            .get_offset(point.line_idx)
            .expect("couldn't get line offset")
            .into();
        line_offset + point.byte_offset
    }

    #[track_caller]
    #[inline]
    pub(crate) fn call<R: 'static>(
        &self,
        fun: impl FnOnce(api::Buffer) -> R + 'static,
    ) -> R {
        let out = Shared::<Option<R>>::new(None);
        let buf = self.inner();
        buf.clone()
            .call({
                let out = out.clone();
                move |()| out.set(Some(fun(buf)))
            })
            .expect("couldn't call function in buffer");
        out.with_mut(Option::take).expect("function wasn't called")
    }

    #[inline]
    pub(crate) fn current(events: &'a Shared<Events>) -> Self {
        Self::new(BufferId::of_focused(), events)
    }

    #[inline]
    pub(crate) fn events(&self) -> Shared<Events> {
        self.events.clone()
    }

    #[track_caller]
    #[inline]
    pub(crate) fn get_name(self) -> PathBuf {
        self.inner().get_name().expect("buffer exists")
    }

    /// Returns the text in the given point range.
    #[track_caller]
    #[inline]
    pub(crate) fn get_text_in_point_range(
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

    #[inline]
    pub(crate) fn inner(&self) -> api::Buffer {
        debug_assert!(self.id.is_valid());
        self.id.into()
    }

    #[track_caller]
    #[inline]
    pub(crate) fn is_focused(self) -> bool {
        api::Window::current().get_buf().expect("window is valid")
            == self.inner()
    }

    #[inline]
    pub(crate) fn new(id: BufferId, events: &'a Shared<Events>) -> Self {
        debug_assert!(id.is_valid());
        Self { id, events }
    }

    /// Converts the given [`ByteOffset`] to the corresponding [`Point`] in the
    /// buffer.
    #[track_caller]
    #[inline]
    pub(crate) fn point_of_byte_offset(
        self,
        byte_offset: ByteOffset,
    ) -> Point {
        if byte_offset == 0 {
            // byte2line can't handle 0.
            return Point::zero();
        }

        let line_idx = self.call(move |this| {
            let line_idx = api::call_function::<_, usize>(
                    "byte2line",
                    (byte_offset.into_u64() as u32,),
                ).expect("offset is within bounds")
                // byte2line returns 1-based line numbers.
                - 1;

            // Whether the character immediately to the left of the given
            // byte offset is a newline.
            let is_offset_after_newline = this
                .get_offset(line_idx + 1)
                .expect("line index is within bounds")
                == byte_offset;

            // byte2line interprets newlines as being the last character
            // of the previous line instead of starting a new one.
            line_idx + is_offset_after_newline as usize
        });

        let line_byte_offset =
            self.inner().get_offset(line_idx).expect("todo");

        Point { line_idx, byte_offset: byte_offset - line_byte_offset }
    }

    /// Returns the [`Point`] at the end of the buffer.
    #[track_caller]
    #[inline]
    pub(crate) fn point_of_eof(self) -> Point {
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

    /// Replaces the text in the given point range with the new text.
    #[track_caller]
    #[inline]
    pub(crate) fn replace_text_in_point_range(
        &self,
        delete_range: Range<Point>,
        insert_text: &str,
    ) {
        // If the text has a trailing newline, Neovim expects an additional
        // empty line to be included.
        let lines = insert_text
            .lines()
            .chain(insert_text.ends_with('\n').then_some(""));

        self.inner()
            .set_text(
                delete_range.start.line_idx..delete_range.end.line_idx,
                delete_range.start.byte_offset.into(),
                delete_range.end.byte_offset.into(),
                lines,
            )
            .expect("replacing text failed");
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

    #[track_caller]
    #[inline]
    pub(crate) fn selection(&self) -> Option<Range<ByteOffset>> {
        if !self.is_focused() || !api::get_mode().mode.is_select_or_visual() {
            return None;
        }

        let (start, end) = self.call(|_this| {
            let (_bufnum, anchor_row, anchor_col) =
                api::call_function::<_, (u32, usize, usize)>("getpos", ('v',))
                    .expect("couldn't call getpos");

            let (_bufnum, head_row, head_col) =
                api::call_function::<_, (u32, usize, usize)>("getpos", ('.',))
                    .expect("couldn't call getpos");

            let mut anchor = Point {
                line_idx: anchor_row - 1,
                byte_offset: ByteOffset::new(anchor_col),
            };

            let mut head = Point {
                line_idx: head_row - 1,
                byte_offset: ByteOffset::new(head_col),
            };

            // The column of the side of the selection that comes first
            // seems to always be off by one, even if it's surrounded by
            // multi-byte characters.
            if anchor <= head {
                anchor.byte_offset -= 1;
                (anchor, head)
            } else {
                head.byte_offset -= 1;
                (head, anchor)
            }
        });

        Some(self.byte_offset_of_point(start)..self.byte_offset_of_point(end))
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
    pub(crate) fn zero() -> Self {
        Self { line_idx: 0, byte_offset: ByteOffset::new(0) }
    }
}

impl Buffer for NeovimBuffer<'_> {
    type Backend = Neovim;

    #[inline]
    fn byte_len(&self) -> ByteOffset {
        self.byte_offset_of_point(self.point_of_eof())
    }

    #[inline]
    fn edit<R>(&mut self, replacements: R, agent_id: AgentId)
    where
        R: IntoIterator<Item = Replacement>,
    {
        for replacement in replacements {
            self.events.with_mut(|events| {
                if events.contains(&events::OnBytes(self.id())) {
                    events.agent_ids.edited_buffer.insert(self.id(), agent_id);
                }
            });

            let range = replacement.removed_range();
            let deletion_start = self.point_of_byte_offset(range.start);
            let deletion_end = self.point_of_byte_offset(range.end);
            self.replace_text_in_point_range(
                deletion_start..deletion_end,
                replacement.inserted_text(),
            );
        }
    }

    #[inline]
    fn id(&self) -> BufferId {
        self.id
    }

    #[inline]
    fn focus(&mut self, _agent_id: AgentId) {
        api::Window::current()
            .set_buf(&self.inner())
            .expect("buffer is valid");
    }

    #[inline]
    fn for_each_cursor<Fun>(&mut self, mut fun: Fun)
    where
        Fun: FnMut(NeovimCursor),
    {
        if self.is_focused() {
            fun(NeovimCursor::new(*self));
        }
    }

    #[inline]
    fn on_edited<Fun>(&self, mut fun: Fun) -> EventHandle
    where
        Fun: FnMut(&NeovimBuffer, &Edit) + 'static,
    {
        Events::insert(
            self.events.clone(),
            events::OnBytes(self.id()),
            move |(this, edit)| fun(this, edit),
        )
    }

    #[inline]
    fn on_removed<Fun>(&self, mut fun: Fun) -> EventHandle
    where
        Fun: FnMut(BufferId, AgentId) + 'static,
    {
        Events::insert(
            self.events.clone(),
            events::BufUnload(self.id()),
            move |(this, removed_by)| fun(this.id(), removed_by),
        )
    }

    #[inline]
    fn on_saved<Fun>(&self, mut fun: Fun) -> EventHandle
    where
        Fun: FnMut(&NeovimBuffer, AgentId) + 'static,
    {
        Events::insert(
            self.events.clone(),
            events::BufWritePost(self.id()),
            move |(this, saved_by)| fun(this, saved_by),
        )
    }

    #[inline]
    fn path(&self) -> Cow<'_, AbsPath> {
        // self.get_name().to_string_lossy().into_owned().into()
        todo!();
    }
}

impl From<NeovimBuffer<'_>> for api::Buffer {
    #[inline]
    fn from(buf: NeovimBuffer) -> Self {
        buf.id().into()
    }
}

impl From<api::Buffer> for BufferId {
    #[inline]
    fn from(buf: api::Buffer) -> Self {
        Self(buf.handle())
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

impl PartialOrd for Point {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.line_idx
            .cmp(&other.line_idx)
            .then(self.byte_offset.cmp(&other.byte_offset))
    }
}
