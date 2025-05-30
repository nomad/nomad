//! TODO: docs.

use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::iter::FusedIterator;
use core::ops::Range;
use std::borrow::Cow;
use std::path::PathBuf;

use compact_str::CompactString;
use ed::backend::{AgentId, Buffer, Chunks, Edit, Replacement};
use ed::fs::AbsPath;
use ed::{ByteOffset, Shared};
use smallvec::smallvec_inline;

use crate::Neovim;
use crate::cursor::NeovimCursor;
use crate::decoration_provider::{self, DecorationProvider};
use crate::events::{self, EventHandle, Events};
use crate::option::{
    Binary,
    EndOfLine,
    FixEndOfLine,
    NeovimOption,
    OptionSet,
};
use crate::oxi::{self, BufHandle, String as NvimString, api, mlua};

/// TODO: docs.
#[derive(Copy, Clone)]
pub struct NeovimBuffer<'a> {
    decoration_provider: &'a DecorationProvider,
    events: &'a Shared<Events>,
    id: BufferId,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BufferId(BufHandle);

/// TODO: docs.
pub struct HighlightRange<'a> {
    buffer: NeovimBuffer<'a>,
    handle: &'a HighlightRangeHandle,
}

/// TODO: docs.
pub struct HighlightRangeHandle {
    inner: decoration_provider::HighlightRange,
}

/// TODO: docs.
pub struct GraphemeOffsets<'a> {
    /// The [`NeovimBuffer`] `Self` iterates over.
    buffer: &'a NeovimBuffer<'a>,

    /// The [`buffer`](Self::buffer)'s byte length.
    byte_len: ByteOffset,

    /// The [`ByteOffset`] `Self` is currently parked at.
    byte_offset: ByteOffset,

    /// The line (or a part of it from/up to some offset) whose grapheme
    /// offsets we're currently iterating over, or `None` if the last call to
    /// [`next()`](Iterator::next) made us move past a newline.
    current_line: Option<NvimString>,

    /// The [`Point`] `Self` is currently parked at.
    ///
    /// This should always refer to the same buffer position as
    /// [`byte_offset`](Self::byte_offset).
    point: Point,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct Point {
    /// The index of the line in the buffer.
    pub(crate) line_idx: usize,

    /// The byte offset in the line.
    pub(crate) byte_offset: ByteOffset,
}

impl<'a> NeovimBuffer<'a> {
    /// TODO: docs.
    #[inline]
    pub fn grapheme_offsets(&self) -> GraphemeOffsets<'_> {
        self.grapheme_offsets_from(0usize.into())
    }

    /// TODO: docs.
    #[inline]
    pub fn grapheme_offsets_from(
        &self,
        byte_offset: ByteOffset,
    ) -> GraphemeOffsets<'_> {
        debug_assert!(byte_offset <= self.byte_len());
        let point = self.point_of_byte(byte_offset);
        GraphemeOffsets {
            buffer: self,
            byte_len: self.byte_len(),
            byte_offset,
            current_line: Some(self.get_line(point.line_idx)),
            point,
        }
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn highlight_range(
        &self,
        byte_range: Range<ByteOffset>,
        highlight_group_name: &str,
    ) -> HighlightRangeHandle {
        debug_assert!(byte_range.start <= byte_range.end);
        debug_assert!(byte_range.end <= self.byte_len());
        let start = self.point_of_byte(byte_range.start);
        let end = self.point_of_byte(byte_range.end);
        HighlightRangeHandle {
            inner: self.decoration_provider.highlight_range(
                self.id(),
                start..end,
                highlight_group_name,
            ),
        }
    }

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
        mut point_range: Range<Point>,
    ) -> CompactString {
        if point_range.is_empty() {
            return CompactString::default();
        }

        // If the buffer's "eol" option is set and the end of the range is
        // after the trailing newline, we need to clamp the end back to the end
        // of the previous line or get_text() will return an out-of-bounds
        // error.
        //
        // For example, if the buffer contains "Hello\nWorld\n" and the point
        // range is `(0, 0)..(2, 0)`, we need to clamp the end to `(1, 5)`.
        //
        // However, because get_text() seems to already clamp offsets in lines,
        // we just set the end to `(line_idx - 1, Integer::MAX)` and let it
        // figure out the offset.
        let needs_to_clamp_end =
            self.is_eol_on() && point_range.end == self.point_of_eof();

        if needs_to_clamp_end {
            point_range.end.line_idx -= 1;
            point_range.end.byte_offset = (oxi::Integer::MAX as usize).into();
        }

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

        if needs_to_clamp_end {
            text.push('\n');
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
    pub(crate) fn new(
        id: BufferId,
        decoration_provider: &'a DecorationProvider,
        events: &'a Shared<Events>,
    ) -> Self {
        debug_assert!(id.is_valid());
        Self { id, decoration_provider, events }
    }

    /// Converts the given [`ByteOffset`] to the corresponding [`Point`] in the
    /// buffer.
    #[track_caller]
    #[inline]
    pub(crate) fn point_of_byte(self, byte_offset: ByteOffset) -> Point {
        debug_assert!(byte_offset <= self.byte_len());

        if byte_offset == 0 {
            // byte2line() can't handle 0.
            return Point::zero();
        } else if byte_offset == 1 && self.byte_len() == 1 {
            // byte2line() has a bug where it returns -1 if the buffer's
            // "memline" (i.e. the object that stores its contents in memory)
            // is not initialized.
            //
            // Because the memline seems to be lazily initialized when the user
            // first edits the buffer, byte2line() will always return -1 on
            // newly created buffers.
            //
            // I brought this up here
            // https://github.com/neovim/neovim/issues/34199, but it was almost
            // immediately closed as a "wontfix" for reasons that are still
            // completely opaque to me.
            //
            // The TLDR of that issue is that the maintainers are not only not
            // willing to fix the bug, but they don't even recognize it as
            // such, so we have to handle it ourselves.
            //
            // Unfortunately there's no public API to check if a memline is
            // initialized, however we can use the fact that in a buffer with
            // an uninitialized memline there can only be up to two possible
            // valid byte offsets: 0 (which we already checked), and — if "eol"
            // is set — 1.
            return if self.is_eol_on() {
                Point::new(1, 0)
            } else {
                Point::new(0, 1)
            };
        }

        let line_idx = self.call(move |this| {
            let line_idx = api::call_function::<_, usize>(
                    "byte2line",
                    (byte_offset.into_u64() as u32,),
                ).expect("offset is within bounds")
                // byte2line() returns 1-based line numbers.
                - 1;

            // Whether the character immediately to the left of the given
            // byte offset is a newline.
            let is_offset_after_newline = this
                .get_offset(line_idx + 1)
                .expect("line index is within bounds")
                == byte_offset;

            // byte2line() interprets newlines as being the last character
            // of the line they end instead of starting a new one.
            line_idx + is_offset_after_newline as usize
        });

        let line_byte_offset = self
            .inner()
            .get_offset(line_idx)
            .expect("line index is within bounds");

        Point::new(line_idx, usize::from(byte_offset) - line_byte_offset)
    }

    /// Returns the [`Point`] at the end of the buffer.
    #[track_caller]
    #[inline]
    pub(crate) fn point_of_eof(self) -> Point {
        let num_rows = self.inner().line_count().expect("buffer is valid");

        let is_eol_on = self.is_eol_on();

        let num_lines = num_rows - 1 + is_eol_on as usize;

        let last_line_len =
            if is_eol_on { 0 } else { self.line_len(num_rows - 1).into() };

        Point::new(num_lines, last_line_len)
    }

    /// Replaces the text in the given point range with the new text.
    #[track_caller]
    #[inline]
    pub(crate) fn replace_text_in_point_range(
        &self,
        mut delete_range: Range<Point>,
        insert_text: &str,
    ) {
        debug_assert!(delete_range.start <= delete_range.end);
        debug_assert!(delete_range.end <= self.point_of_eof());

        if delete_range.is_empty() {
            return;
        }

        // We need to clamp the end in the same way we do in
        // get_text_in_point_range(). See that comment for more details.
        let needs_to_clamp_end =
            self.is_eol_on() && delete_range.end == self.point_of_eof();

        if needs_to_clamp_end {
            let end = &mut delete_range.end;
            end.line_idx -= 1;
            end.byte_offset = self.line_len(end.line_idx);
        }

        // If the text has a trailing newline, Neovim expects an additional
        // empty line to be included.
        let lines = insert_text
            .lines()
            .chain(insert_text.ends_with('\n').then_some(""));

        // If we needed to clamp the end of the range, it means the user also
        // wanted to delete the trailing newline.
        //
        // However, Neovim made the unfortunate design decision of assuming
        // that every buffer ends in `\n`, and all the buffer-editing APIs will
        // return an error if you try to set the end position of the deleted
        // range past it.
        //
        // The only way to get around this is to unset both the "eol" and
        // "fixeol" options, which acts as if the trailing newline was deleted,
        // even marking the buffer as "modified".
        //
        // The drawback of this approach is that the trailing newline won't be
        // re-inserted the next time the buffer is saved, unless the user
        // manually re-enables either "eol", "fixeol", or both.
        //
        // While this sucks, it sucks less than not respecting the user's
        // intent.
        let should_unset_eol_fixeol = needs_to_clamp_end;

        if should_unset_eol_fixeol {
            // If someone is receiving edit events for this buffer, we need to
            // mark this buffer's ID as the one that just had their trailing
            // newline deleted so that:
            //
            // 1) in `Self::replacement_of_on_bytes()` we'll know to extend the
            //    deleted range by 1;
            //
            // 2) we'll know to ignore the next `OptionSet` autocommands that
            //    will be triggered when we unset "eol" and "fixeol";
            self.events.with_mut(|events| {
                if events.contains(&events::OnBytes(self.id())) {
                    events.agent_ids.has_just_deleted_trailing_newline =
                        Some(self.id());
                }
            });
        }

        self.inner()
            .set_text(
                delete_range.start.line_idx..delete_range.end.line_idx,
                delete_range.start.byte_offset.into(),
                delete_range.end.byte_offset.into(),
                lines,
            )
            .expect("replacing text failed");

        if should_unset_eol_fixeol {
            let opts = self.into();
            EndOfLine.set(false, &opts);
            FixEndOfLine.set(false, &opts);

            // All the callbacks registered to `OnBytes`, "eol" and "fixeol"
            // have now been executed, so we can cleanup the state.
            self.events.with_mut(|events| {
                events.agent_ids.has_just_deleted_trailing_newline = None;
            });
        }
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

        let has_just_deleted_trailing_newline = self.events.with(|events| {
            events.agent_ids.has_just_deleted_trailing_newline
                == Some(self.id())
        });

        let deletion_start = start_offset.into();

        let deletion_end = (start_offset
            + old_end_len
            // Add 1 if the user just called
            // `Self::replace_text_in_point_range()` with a byte range that
            // included the trailing newline.
            + has_just_deleted_trailing_newline as usize)
            .into();

        let insertion_start =
            Point { line_idx: start_row, byte_offset: start_col.into() };

        let insertion_end = Point {
            line_idx: start_row + new_end_row,
            byte_offset: (start_col * (new_end_row == 0) as usize
                + new_end_col)
                .into(),
        };

        Replacement::new(
            deletion_start..deletion_end,
            &*self.get_text_in_point_range(insertion_start..insertion_end),
        )
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
    ///
    /// Note that if you just want to know the *length* of the line, you should
    /// use [`line_len()`](Self::line_len) instead.
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
    fn is_eol_on(&self) -> bool {
        let opts = self.into();
        is_eol_on(
            EndOfLine.get(&opts),
            FixEndOfLine.get(&opts),
            Binary.get(&opts),
        )
    }

    /// Returns the byte length of the line at the given index, *without* any
    /// trailing newline character.
    ///
    /// This is equivalent to `self.get_line(line_idx).len()`, but faster.
    #[inline]
    fn line_len(&self, line_idx: usize) -> ByteOffset {
        // TODO: benchmark whether this is actually faster than
        // `self.get_line(line_idx).len()`.

        let row = (line_idx + 1) as oxi::Integer;

        let col = api::call_function::<_, usize>(
            "col",
            oxi::Array::from_iter([
                oxi::Object::from(row),
                oxi::Object::from("$"),
            ]),
        )
        .expect("could not call col()");

        (col - 1).into()
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

        let (start, end) =
            if anchor <= head { (anchor, head) } else { (head, anchor) };

        let end_offset = {
            let offset = self.byte_of_point(end);
            // The length of the last selected grapheme is not included in the
            // coordinates returned by getpos(), so we need to add it
            // ourselves.
            self.grapheme_offsets_from(offset).next().unwrap_or(offset)
        };

        self.byte_of_point(start)..end_offset
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

impl<'a> HighlightRange<'a> {
    /// TODO: docs.
    #[inline]
    pub fn buffer(&self) -> NeovimBuffer<'_> {
        self.buffer
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn r#move(&self, byte_range: Range<ByteOffset>) {
        debug_assert!(byte_range.start <= byte_range.end);
        debug_assert!(byte_range.end <= self.buffer().byte_len());
        let start = self.buffer().point_of_byte(byte_range.start);
        let end = self.buffer().point_of_byte(byte_range.end);
        self.handle.inner.r#move(start..end);
    }

    /// TODO: docs.
    #[inline]
    pub fn set_highlight_group(&self, highlight_group_name: &str) {
        self.handle.inner.set_hl_group(highlight_group_name);
    }

    #[inline]
    pub(crate) fn new(
        buffer: NeovimBuffer<'a>,
        handle: &'a HighlightRangeHandle,
    ) -> Self {
        debug_assert_eq!(buffer.id(), handle.buffer_id());
        Self { buffer, handle }
    }
}

impl HighlightRangeHandle {
    #[inline]
    pub(crate) fn buffer_id(&self) -> BufferId {
        self.inner.buffer_id()
    }
}

impl Point {
    #[inline]
    pub(crate) fn new(line_idx: usize, byte_offset: usize) -> Self {
        Self { line_idx, byte_offset: byte_offset.into() }
    }

    #[inline]
    pub(crate) fn zero() -> Self {
        Self::new(0, 0)
    }
}

impl Buffer for NeovimBuffer<'_> {
    type Backend = Neovim;

    #[inline]
    fn byte_len(&self) -> ByteOffset {
        let buf = self.inner();
        let line_count = buf.line_count().expect("buffer is valid");
        buf.get_offset(line_count).expect("buffer is valid").into()
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
    fn on_edited<Fun>(&self, fun: Fun) -> EventHandle
    where
        Fun: FnMut(&NeovimBuffer, &Edit) + 'static,
    {
        let fun = Shared::<Fun>::new(fun);

        let on_bytes_handle =
            Events::insert(self.events.clone(), events::OnBytes(self.id()), {
                let fun = fun.clone();
                move |(this, edit)| fun.with_mut(|fun| fun(this, edit))
            });

        // Setting/unsetting the right combination of "endofline",
        // "fixendofline" and "binary" behaves as if deleting/inserting a
        // trailing newline, so we need to listen for all three events.

        let buffer_id = self.id();

        let end_of_line_set_handle = Events::insert(
            self.events.clone(),
            OptionSet::<EndOfLine>::new(),
            {
                let fun = fun.clone();
                move |(buf, &old_value, &new_value)| {
                    let buf = buf.expect("endofline is buffer-local");
                    if buf.id() != buffer_id {
                        return;
                    }

                    let opts = (&buf).into();
                    let fix_end_of_line = FixEndOfLine.get(&opts);
                    let binary = Binary.get(&opts);

                    react_to_eol_changes(
                        &buf,
                        (old_value, new_value),
                        (fix_end_of_line, fix_end_of_line),
                        (binary, binary),
                        &fun,
                    );
                }
            },
        );

        let fix_end_of_line_set_handle = Events::insert(
            self.events.clone(),
            OptionSet::<FixEndOfLine>::new(),
            {
                let fun = fun.clone();
                move |(buf, &old_value, &new_value)| {
                    let buf = buf.expect("fixendofline is buffer-local");
                    if buf.id() != buffer_id {
                        return;
                    }

                    let opts = (&buf).into();
                    let end_of_line = EndOfLine.get(&opts);
                    let binary = Binary.get(&opts);

                    react_to_eol_changes(
                        &buf,
                        (end_of_line, end_of_line),
                        (old_value, new_value),
                        (binary, binary),
                        &fun,
                    );
                }
            },
        );

        let binary_set_handle =
            Events::insert(self.events.clone(), OptionSet::<Binary>::new(), {
                let fun = fun.clone();
                move |(buf, &old_value, &new_value)| {
                    let buf = buf.expect("binary is buffer-local");
                    if buf.id() != buffer_id {
                        return;
                    }

                    let opts = (&buf).into();
                    let end_of_line = EndOfLine.get(&opts);
                    let fix_end_of_line = FixEndOfLine.get(&opts);

                    react_to_eol_changes(
                        &buf,
                        (end_of_line, end_of_line),
                        (fix_end_of_line, fix_end_of_line),
                        (old_value, new_value),
                        &fun,
                    );
                }
            });

        on_bytes_handle
            .merge(end_of_line_set_handle)
            .merge(fix_end_of_line_set_handle)
            .merge(binary_set_handle)
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

impl From<api::Buffer> for BufferId {
    #[inline]
    fn from(buf: api::Buffer) -> Self {
        Self(buf.handle())
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

impl Iterator for GraphemeOffsets<'_> {
    type Item = ByteOffset;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // We're at the end of the buffer.
        if self.byte_offset == self.byte_len {
            return None;
        }

        let line_from_offset = &self
            .current_line
            .get_or_insert_with(|| self.buffer.get_line(self.point.line_idx))
            .as_bytes()[usize::from(self.point.byte_offset)..];

        if line_from_offset.is_empty() {
            // We're at the end of the current line, so the next grapheme
            // must be a newline character.
            self.byte_offset += 1;
            self.point.line_idx += 1;
            self.point.byte_offset = 0usize.into();
            self.current_line = None;
            Some(self.byte_offset)
        } else {
            // TODO: avoid allocating a new NvimString every time.
            let grapheme_len = api::call_function::<_, usize>(
                "byteidx",
                (NvimString::from_bytes(line_from_offset), 1),
            )
            .expect("couldn't call byteidx()");
            self.byte_offset += grapheme_len;
            self.point.byte_offset += grapheme_len;
            Some(self.byte_offset)
        }
    }
}

impl FusedIterator for GraphemeOffsets<'_> {}

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

impl From<BufferId> for oxi::Object {
    #[inline]
    fn from(buf_id: BufferId) -> Self {
        buf_id.0.into()
    }
}

impl fmt::Debug for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Point")
            .field(&self.line_idx)
            .field(&usize::from(self.byte_offset))
            .finish()
    }
}

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

fn is_eol_on(eol: bool, fixeol: bool, binary: bool) -> bool {
    eol || (fixeol && !binary)
}

fn react_to_eol_changes(
    buf: &NeovimBuffer,
    (old_eol, new_eol): (bool, bool),
    (old_fixeol, new_fixeol): (bool, bool),
    (old_binary, new_binary): (bool, bool),
    fun: &Shared<impl FnMut(&NeovimBuffer, &Edit)>,
) {
    // If 'endofline' and 'fixendofline' were turned off by us in
    // `NeovimBuffer::replace_text_in_point_range()`, the callback registered
    // to `OnBytes` already took care of including the trailing newline in the
    // deleted range, so we don't need to do anything here.
    if buf.events.with(|events| {
        events.agent_ids.has_just_deleted_trailing_newline == Some(buf.id())
    }) {
        return;
    }

    let was_eol_on = is_eol_on(old_eol, old_fixeol, old_binary);

    let is_eol_on = is_eol_on(new_eol, new_fixeol, new_binary);

    let byte_len = buf.byte_len();

    let replacement = match (was_eol_on, is_eol_on) {
        // The trailing newline was deleted.
        (true, false) => Replacement::removal(byte_len..byte_len + 1),
        // The trailing newline was added.
        (false, true) => Replacement::insertion(byte_len - 1, "\n"),
        _ => return,
    };

    let edit = Edit {
        made_by: AgentId::UNKNOWN,
        replacements: smallvec_inline![replacement],
    };

    fun.with_mut(|fun| fun(buf, &edit));
}
