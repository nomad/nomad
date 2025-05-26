//! TODO: docs.

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::num::NonZeroUsize;
use core::ops::Range;
use std::borrow::Cow;
use std::path::PathBuf;

use compact_str::CompactString;
use ed::backend::{AgentId, Buffer, Chunks, Edit, Replacement};
use ed::fs::AbsPath;
use ed::{ByteOffset, Shared};

use crate::Neovim;
use crate::cursor::NeovimCursor;
use crate::events::{self, EventHandle, Events};
use crate::oxi::{self, BufHandle, String as NvimString, api, mlua};

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
    pub(crate) fn byte_of_point(self, point: Point) -> ByteOffset {
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
    pub(crate) fn point_of_byte(self, byte_offset: ByteOffset) -> Point {
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
        let line_len = usize::from(self.line_len());

        let byte_offset = if self.has_trailing_newline() {
            0usize.into()
        } else {
            self.byte_of_line(line_len) - self.byte_of_line(line_len - 1)
        };

        Point { line_idx: line_len - 1, byte_offset }
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
        if !self.is_focused() {
            return None;
        }

        let mode = api::get_mode().mode;

        if mode.is_select_by_character() || mode.is_visual_by_character() {
            Some(self.selection_by_character())
        } else if mode.is_select_by_line() || mode.is_visual_by_line() {
            Some(self.selection_by_line())
        } else if mode.is_select_blockwise() || mode.is_visual_blockwise() {
            // We don't yet support visual block mode because the corresponding
            // selection could span several disjoint byte ranges.
            None
        } else {
            None
        }
    }

    /// Returns the [`ByteOffset`] corresponding to the given line offset.
    ///
    /// # Panics
    ///
    /// Panics if the line offset is out of bounds (i.e. greater than
    /// [`line_len()`](Self::line_len)).
    #[inline]
    fn byte_of_line(&self, line_offset: usize) -> ByteOffset {
        // get_offset() already takes care of only counting the final newline
        // if `eol` is enabled.
        self.inner()
            .get_offset(line_offset)
            .expect("line index out of bounds")
            .into()
    }

    /// Returns the contents of the line at the given index, *without* any
    /// trailing newline character.
    #[inline]
    fn get_line(&self, line_idx: usize) -> NvimString {
        api::call_function(
            "getbufoneline",
            (self.id(), (line_idx + 1) as oxi::Integer),
        )
        .expect("could not call getbufoneline()")
    }

    /// TODO: docs.
    #[inline]
    fn has_trailing_newline(&self) -> bool {
        let bool_opt = |opt_name: &str| {
            api::get_option_value::<bool>(
                opt_name,
                &api::opts::OptionOpts::builder().buffer(self.inner()).build(),
            )
            .expect("buffer is valid")
        };

        bool_opt("fixeol") && (bool_opt("eol") || !bool_opt("binary"))
    }

    /// Returns the number of lines in the buffer, which is defined as being
    /// one more than the number of newline characters in the buffer.
    ///
    /// Note that an empty empty buffer will a `line_len` of 1.
    #[inline]
    fn line_len(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.inner().line_count().expect("buffer is valid"))
            .expect("the output of line_count() is always >=1 ")
            .saturating_add(self.has_implicit_trailing_newline() as usize)
    }

    /// Returns the selected byte range in the buffer, assuming:
    ///
    /// - `Self` is focused;
    /// - the user is in character-wise visual or select mode;
    ///
    /// # Panics
    ///
    /// Panics if either one of those assumptions is not true.
    #[inline]
    fn selection_by_character(&self) -> Range<ByteOffset> {
        debug_assert!(self.is_focused());
        debug_assert!({
            let mode = api::get_mode().mode;
            mode.is_select_by_character() || mode.is_visual_by_character()
        });

        let (_bufnum, anchor_row, anchor_col) =
            api::call_function::<_, (u32, usize, usize)>("getpos", ('v',))
                .expect("couldn't call getpos");

        let (_bufnum, head_row, head_col) =
            api::call_function::<_, (u32, usize, usize)>("getpos", ('.',))
                .expect("couldn't call getpos");

        let anchor = Point {
            line_idx: anchor_row - 1,
            byte_offset: ByteOffset::new(anchor_col - 1),
        };

        let head = Point {
            line_idx: head_row - 1,
            byte_offset: ByteOffset::new(head_col - 1),
        };

        let (start, mut end) =
            if anchor <= head { (anchor, head) } else { (head, anchor) };

        // The length of the last selected grapheme is never included in the
        // coordinates returned by getpos(), so we need to add it ourselves.
        let final_grapheme_len = {
            let end_line = self.get_line(end.line_idx);
            let cursor_to_eol = &end_line.as_bytes()[end.byte_offset.into()..];
            String::from_utf8_lossy(cursor_to_eol)
                // FIXME: use graphemes.
                .chars()
                .next()
                .map(char::len_utf8)
                // The cursor is already at EOL, so the next grapheme must
                // be a \n.
                .unwrap_or(1)
        };

        end.byte_offset += final_grapheme_len;

        // Neovim always allows you to select one more character past the end
        // of the line, which is usually interpreted as having selected the
        // following newline.
        //
        // Clearly that doesn't work if you're already at the end of the file.
        end = end.min(self.point_of_eof());

        self.byte_of_point(start)..self.byte_of_point(end)
    }

    /// Returns the selected byte range in the buffer, assuming:
    ///
    /// - `Self` is focused;
    /// - the user is in line-wise visual or select mode;
    ///
    /// # Panics
    ///
    /// Panics if either one of those assumptions in not true.
    #[inline]
    fn selection_by_line(&self) -> Range<ByteOffset> {
        debug_assert!(self.is_focused());
        debug_assert!({
            let mode = api::get_mode().mode;
            mode.is_select_by_line() || mode.is_visual_by_line()
        });

        let anchor_row = api::call_function::<_, usize>("line", ('v',))
            .expect("couldn't call line()");

        let head_row = api::call_function::<_, usize>("line", ('.',))
            .expect("couldn't call line()");

        let (start_row, end_row) = if anchor_row <= head_row {
            (anchor_row, head_row)
        } else {
            (head_row, anchor_row)
        };

        let start_offset = self.byte_of_line(start_row - 1);

        // Neovim always allows you to select one more character past the end
        // of the line, which is usually interpreted as having selected the
        // following newline.
        //
        // Clearly that doesn't work if you're already at the end of the file.
        let end_offset = self.byte_of_line(end_row).min(self.byte_len());

        start_offset..end_offset
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
        self.byte_of_line(self.line_len().into())
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
            let deletion_start = self.point_of_byte(range.start);
            let deletion_end = self.point_of_byte(range.end);
            self.replace_text_in_point_range(
                deletion_start..deletion_end,
                replacement.inserted_text(),
            );
        }
    }

    #[inline]
    fn get_text(&self, byte_range: Range<ByteOffset>) -> impl Chunks {
        let start = self.point_of_byte(byte_range.start);
        let end = self.point_of_byte(byte_range.end);
        self.get_text_in_point_range(start..end)
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

impl From<BufferId> for oxi::Object {
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
