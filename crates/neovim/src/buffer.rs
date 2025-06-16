//! TODO: docs.

use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::iter::FusedIterator;
use core::ops::Range;
use std::borrow::Cow;
use std::path::PathBuf;

use compact_str::CompactString;
use ed::fs::AbsPath;
use ed::{AgentId, Buffer, ByteOffset, Chunks, Edit, Replacement, Shared};
use smallvec::{SmallVec, smallvec_inline};

use crate::Neovim;
use crate::convert::Convert;
use crate::cursor::NeovimCursor;
use crate::decoration_provider::{self, DecorationProvider};
use crate::events::{self, EventHandle, Events};
use crate::option::{NeovimOption, UneditableEndOfLine};
use crate::oxi::{self, BufHandle, String as NvimString, api, mlua};

/// TODO: docs.
#[derive(Copy, Clone)]
pub struct NeovimBuffer<'a> {
    id: BufferId,
    events: &'a Shared<Events>,
    state: &'a BuffersState,
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

#[derive(Clone)]
pub(crate) struct BuffersState {
    decoration_provider: DecorationProvider,
    on_bytes_replacement_extend_deletion_end_by_one: Shared<bool>,
    on_bytes_replacement_insertion_starts_at_next_line: Shared<bool>,
    skip_next_uneditable_eol: Shared<bool>,
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
        self.grapheme_offsets_from(0)
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
            inner: self.state.decoration_provider.highlight_range(
                self.id(),
                start..end,
                highlight_group_name,
            ),
        }
    }

    /// Returns an iterator over the `(byte_range, hl_groups)` tuples of all
    /// highlight ranges set on this buffer.
    #[inline]
    pub fn highlight_ranges(
        &self,
    ) -> impl Iterator<Item = (Range<ByteOffset>, SmallVec<[String; 1]>)> {
        let opts = api::opts::GetExtmarksOpts::builder()
            .details(true)
            .ty("highlight")
            .build();

        self.inner()
            .get_extmarks(
                api::types::GetExtmarksNamespaceId::All,
                Point::zero().into(),
                self.point_of_eof().into(),
                &opts,
            )
            .expect("couldn't get extmarks")
            .map(|(_ns_id, start_row, start_col, maybe_infos)| {
                let infos = maybe_infos.expect("requested details");
                let end_row = infos.end_row.expect("set for hl marks");
                let end_col = infos.end_col.expect("set for hl marks");
                let hl_group = infos.hl_group.expect("set for hl marks");

                let start_point = Point::new(start_row, start_col);
                let end_point = Point::new(end_row, end_col);
                let start = self.byte_of_point(start_point);
                let end = self.byte_of_point(end_point);
                (start..end, hl_group.convert())
            })
    }

    /// Converts the given [`Point`] to the corresponding [`ByteOffset`] in the
    /// buffer.
    #[track_caller]
    #[inline]
    pub(crate) fn byte_of_point(&self, point: Point) -> ByteOffset {
        let line_offset = self
            .inner()
            .get_offset(point.line_idx)
            .expect("couldn't get line offset");
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
    pub(crate) fn get_name(&self) -> PathBuf {
        self.inner().get_name().expect("buffer exists")
    }

    /// Returns the text in the given point range.
    #[track_caller]
    #[inline]
    pub(crate) fn get_text_in_point_range(
        &self,
        mut point_range: Range<Point>,
    ) -> CompactString {
        debug_assert!(point_range.start <= point_range.end);
        debug_assert!(point_range.end <= self.point_of_eof());

        // If the buffer has an uneditable eol and the end of the range
        // includes it, we need to clamp the end back to the end of the
        // previous line or get_text() will return an out-of-bounds error.
        //
        // For example, if the buffer contains "Hello\nWorld\n" and the point
        // range is `(0, 0)..(2, 0)`, we need to clamp the end to `(1, 5)`.
        //
        // However, because get_text() seems to already clamp offsets in lines,
        // we just set the end's line offset to `(line_idx - 1, Integer::MAX)`
        // and let get_text() figure it out.
        let should_clamp_end =
            self.is_point_after_uneditable_eol(point_range.end);

        if should_clamp_end {
            point_range.end.line_idx -= 1;
            point_range.end.byte_offset = oxi::Integer::MAX as usize;

            // The original start was <= than the end, so if it's now greater
            // it means they were both equal to the point of eof, i.e. the
            // range was empty.
            if point_range.start > point_range.end {
                return CompactString::default();
            }
        }

        let lines = self
            .inner()
            .get_text(
                point_range.start.line_idx..point_range.end.line_idx,
                point_range.start.byte_offset,
                point_range.end.byte_offset,
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

        if should_clamp_end {
            text.push('\n');
        }

        text
    }

    #[inline]
    pub(crate) fn inner(&self) -> api::Buffer {
        debug_assert!(self.id.is_valid());
        self.id.into()
    }

    #[inline]
    pub(crate) fn is_focused(&self) -> bool {
        api::Window::current().get_buf().expect("window is valid")
            == self.inner()
    }

    #[inline]
    pub(crate) fn new(
        id: BufferId,
        events: &'a Shared<Events>,
        state: &'a BuffersState,
    ) -> Self {
        debug_assert!(id.is_valid());
        Self { id, events, state }
    }

    /// Converts the given [`ByteOffset`] to the corresponding [`Point`] in the
    /// buffer.
    #[track_caller]
    #[inline]
    pub(crate) fn point_of_byte(&self, byte_offset: ByteOffset) -> Point {
        debug_assert!(byte_offset <= self.byte_len());

        // byte2line(1) has a bug where it returns -1 if the buffer's "memline"
        // (i.e. the object that stores its contents in memory) is not
        // initialized.
        //
        // Because the memline seems to be lazily initialized when the user
        // first edits the buffer, byte2line(1) will always return -1 on newly
        // created, empty buffers.
        //
        // I brought this up here
        // https://github.com/neovim/neovim/issues/34199, but it was almost
        // immediately closed as a "wontfix" for reasons that are still
        // completely opaque to me.
        //
        // The TLDR of that issue is that the maintainers are not only not
        // willing to fix the bug, but they don't even recognize it as such, so
        // we have to special-case it.
        if byte_offset == 0 {
            return Point::zero();
        }
        // byte2line() always returns -1 if the buffer has an uneditable eol
        // and the byte offset is past it.
        else if byte_offset == self.byte_len() {
            return self.point_of_eof();
        }

        let line_idx = self.call(move |this| {
            let line_idx = api::call_function::<_, usize>(
                    "byte2line",
                    (byte_offset as u32 + 1,),
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

        Point::new(line_idx, byte_offset - line_byte_offset)
    }

    /// Returns the [`Point`] at the end of the buffer.
    #[track_caller]
    #[inline]
    pub(crate) fn point_of_eof(&self) -> Point {
        // Workaround for https://github.com/neovim/neovim/issues/34272.
        if self.is_empty() {
            return Point::zero();
        }

        let num_lines = self.inner().line_count().expect("buffer is valid");

        let has_uneditable_eol = self.has_uneditable_eol();

        let num_lines = num_lines - 1 + has_uneditable_eol as usize;

        let last_line_len =
            if has_uneditable_eol { 0 } else { self.line_len(num_lines - 1) };

        Point::new(num_lines, last_line_len)
    }

    /// Replaces the text in the given point range with the new text.
    ///
    /// # Panics
    ///
    /// Panics if the replacement is a no-op, i.e. if both the range to delete
    /// and the text to insert are empty.
    #[track_caller]
    #[inline]
    pub(crate) fn replace_text_in_point_range(
        &self,
        mut delete_range: Range<Point>,
        insert_text: &str,
        agent_id: AgentId,
    ) {
        debug_assert!(delete_range.start <= delete_range.end);
        debug_assert!(delete_range.end <= self.point_of_eof());
        debug_assert!(!delete_range.is_empty() || !insert_text.is_empty());

        // If the buffer has an uneditable eol, we might need to clamp the
        // points of the deleted range in the same way we do in
        // get_text_in_point_range(). See that comment for more details.

        let should_clamp_end =
            self.is_point_after_uneditable_eol(delete_range.end);

        if should_clamp_end {
            let end = &mut delete_range.end;
            end.line_idx -= 1;
            end.byte_offset = self.line_len(end.line_idx);
        }

        let should_clamp_start = delete_range.start > delete_range.end;

        if should_clamp_start {
            // The original start was <= than the end, so if we need to clamp
            // the start it means we just clamped the end.
            debug_assert!(should_clamp_end);
            delete_range.start = delete_range.end;
        }

        // If we needed to clamp the end of the range it means the user wants
        // to delete the trailing newline or insert text after it.
        //
        // However, Neovim made the unfortunate design decision of assuming
        // that every buffer ends in `\n`, and all the buffer-editing APIs will
        // return an error if you try to set the end position of the deleted
        // range past it.
        //
        // The only way to get around this is to unset the uneditable eol,
        // which acts as if the trailing newline was deleted, even marking the
        // buffer as "modified".
        //
        // The drawback of this approach is that the trailing newline won't be
        // re-inserted the next time the buffer is saved, unless the user
        // manually re-enables either "eol", "fixeol", or both.
        //
        // While this sucks, it sucks less than not respecting the user's
        // intent.
        let should_unset_uneditable_eol = should_clamp_end;

        // If we clamped the start it means the replacement was a pure
        // insertion after the uneditable eol (e.g. buffer contains "Hello\n",
        // replacement is delete 6..6, insert "World").
        let insert_after_uneditable_eol = should_clamp_start;

        if should_unset_uneditable_eol {
            self.events.with_mut(|events| {
                if !events.contains(&events::OnBytes(self.id())) {
                    return;
                }

                // We're about to:
                //
                // 1) unset the buffer's uneditable eol setting, which will
                //    trigger a set event on UneditableEndOfLine;
                // 2) call set_text(), which will trigger an OnBytes event;
                //
                // Since both events are triggered by the same replacement, the
                // edit event handlers should only be called once, so we skip
                // the next UneditableEndOfLine event if OnBytes is triggered.
                let is_on_bytes_triggered =
                    !delete_range.is_empty() || !insert_text.is_empty();

                if is_on_bytes_triggered {
                    self.state.skip_next_uneditable_eol.set(true);

                    // Extend the end of the deleted range by one byte
                    // to account for having deleted the trailing newline.
                    self.state
                        .on_bytes_replacement_extend_deletion_end_by_one
                        .set(true);

                    if insert_after_uneditable_eol {
                        // Make the inserted text start at the next line to
                        // ignore the newline that we're about to re-add.
                        self.state
                            .on_bytes_replacement_insertion_starts_at_next_line
                            .set(true);
                    }
                } else {
                    // OnBytes is not triggered, so set the AgentId that
                    // removed the UneditableEndOfLine because we won't skip
                    // the next event on it.
                    events.agent_ids.set_uneditable_eol.set(agent_id);
                }
            });

            UneditableEndOfLine.set(false, &self.into());
        }

        let lines =
            // To insert after the uneditable eol we first had to disable it,
            // so we need to re-add a newline to the buffer to balance it out.
            insert_after_uneditable_eol.then_some("").into_iter()
            .chain(insert_text.lines())
            // If the text has a trailing newline, Neovim expects an additional
            // empty line to be included.
            .chain(insert_text.ends_with('\n').then_some(""));

        self.inner()
            .set_text(
                delete_range.start.line_idx..delete_range.end.line_idx,
                delete_range.start.byte_offset,
                delete_range.end.byte_offset,
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

        let should_extend_end =
            self.state.on_bytes_replacement_extend_deletion_end_by_one.take();

        let should_start_at_next_line = self
            .state
            .on_bytes_replacement_insertion_starts_at_next_line
            .take();

        let should_extend_start =
            should_extend_end && should_start_at_next_line;

        let deletion_start = start_offset + should_extend_start as usize;

        let deletion_end =
            start_offset + old_end_len + should_extend_end as usize;

        let mut insertion_start =
            Point { line_idx: start_row, byte_offset: start_col };

        if should_start_at_next_line {
            insertion_start.line_idx += 1;
            insertion_start.byte_offset = 0;
        }

        let insertion_end = Point {
            line_idx: start_row + new_end_row,
            byte_offset: start_col * (new_end_row == 0) as usize + new_end_col,
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
        self.inner().get_offset(line_offset).expect("line index out of bounds")
    }

    /// Returns the contents of the line at the given index, *without* any
    /// trailing newline character.
    ///
    /// Note that if you just want to know the *length* of the line, you should
    /// use [`line_len()`](Self::line_len) instead.
    #[inline]
    fn get_line(&self, line_idx: usize) -> NvimString {
        let buffer_id = self.id();
        self.inner()
            .call(move |()| {
                api::call_function(
                    "getbufoneline",
                    (buffer_id, (line_idx + 1) as oxi::Integer),
                )
            })
            .expect("could not call getbufoneline()")
    }

    /// TODO: docs.
    #[inline]
    fn has_uneditable_eol(&self) -> bool {
        UneditableEndOfLine.get(&self.into())
    }

    #[inline]
    pub(crate) fn is_point_after_uneditable_eol(&self, point: Point) -> bool {
        !self.is_empty()
            && self.has_uneditable_eol()
            && point == self.point_of_eof()
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

        let col: usize = self
            .inner()
            .call(move |()| {
                api::call_function(
                    "col",
                    (oxi::Array::from_iter([
                        oxi::Object::from(row),
                        oxi::Object::from("$"),
                    ]),),
                )
            })
            .expect("could not call col()");

        col - 1
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

        let anchor =
            Point { line_idx: anchor_row - 1, byte_offset: anchor_col - 1 };

        let head = Point { line_idx: head_row - 1, byte_offset: head_col - 1 };

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
    /// Returns the underlying buffer number of this [`BufferId`].
    #[inline]
    pub fn bufnr(self) -> u32 {
        self.0 as u32
    }

    /// Returns the [`BufferId`] of the currently focused buffer.
    #[inline]
    pub fn of_focused() -> Self {
        Self::new(api::Buffer::current())
    }

    #[inline]
    pub(crate) fn is_valid(self) -> bool {
        api::Buffer::from(self).is_valid()
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

impl BuffersState {
    #[inline]
    pub(crate) fn new(decoration_provider: DecorationProvider) -> Self {
        Self {
            decoration_provider,
            on_bytes_replacement_extend_deletion_end_by_one: Default::default(
            ),
            on_bytes_replacement_insertion_starts_at_next_line:
                Default::default(),
            skip_next_uneditable_eol: Default::default(),
        }
    }
}

impl Point {
    #[inline]
    pub(crate) fn new(line_idx: usize, byte_offset: usize) -> Self {
        Self { line_idx, byte_offset }
    }

    #[inline]
    pub(crate) fn zero() -> Self {
        Self::new(0, 0)
    }
}

impl Buffer for NeovimBuffer<'_> {
    type Editor = Neovim;

    #[inline]
    fn byte_len(&self) -> ByteOffset {
        let buf = self.inner();
        let line_count = buf.line_count().expect("buffer is valid");
        let offset = buf.get_offset(line_count).expect("buffer is valid");
        // Workaround for https://github.com/neovim/neovim/issues/34272.
        if offset == 1 && self.has_uneditable_eol() { 0 } else { offset }
    }

    #[inline]
    fn edit<R>(&mut self, replacements: R, agent_id: AgentId)
    where
        R: IntoIterator<Item = Replacement>,
    {
        for replacement in replacements {
            if replacement.is_no_op() {
                continue;
            }

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
                agent_id,
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
                move |(this, edit)| {
                    fun.with_mut(|fun| fun(this, edit));

                    if this.has_uneditable_eol() {
                        // If the buffer has an uneditable eol, then:
                        //
                        // - if the buffer was empty and text is inserted the
                        // eol "activates", and we should notify the user that
                        // a \n was inserted;
                        //
                        // - if all the text is deleted and the buffer is now
                        // empty the eol "deactivates", and we should notify
                        // the user that a \n was deleted;

                        let buf_len = this.byte_len();
                        let edit_len_delta = edit.byte_delta();
                        let edit_len_delta_abs = edit_len_delta.unsigned_abs();

                        let replacement = if edit_len_delta.is_positive()
                            && edit_len_delta_abs + 1 == buf_len
                        {
                            Replacement::insertion(edit_len_delta_abs, "\n")
                        } else if edit_len_delta.is_negative() && buf_len == 0
                        {
                            Replacement::removal(0..1)
                        } else {
                            return;
                        };

                        let edit = Edit {
                            made_by: AgentId::UNKNOWN,
                            replacements: smallvec_inline![replacement],
                        };

                        fun.with_mut(|fun| fun(this, &edit));
                    }
                }
            });

        // Setting/unsetting the uneditable eol behaves as if
        // deleting/inserting a trailing newline, so we need to react to it.

        let buffer_id = self.id();

        let uneditable_eol_set_handle = Events::insert(
            self.events.clone(),
            UneditableEndOfLine,
            move |(buf, was_set, is_set, set_by)| {
                // Ignore event if setting didn't change, if it changed for a
                // different buffer or if we were told to skip this event.
                if was_set == is_set
                    || buf.id() != buffer_id
                    || buf.state.skip_next_uneditable_eol.take()
                {
                    return;
                }

                let byte_len = buf.byte_len();

                // Eol-settings don't apply on empty buffers.
                if byte_len == 0 {
                    return;
                }

                let replacement = match (was_set, is_set) {
                    // The trailing newline was deleted.
                    (true, false) => {
                        Replacement::removal(byte_len..byte_len + 1)
                    },
                    // The trailing newline was added.
                    (false, true) => {
                        Replacement::insertion(byte_len - 1, "\n")
                    },
                    _ => unreachable!("already checked"),
                };

                let edit = Edit {
                    made_by: set_by,
                    replacements: smallvec_inline![replacement],
                };

                fun.with_mut(|fun| fun(&buf, &edit));
            },
        );

        on_bytes_handle.merge(uneditable_eol_set_handle)
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
        self.0.into_lua(lua)
    }
}

impl Hash for BufferId {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_i32(self.0);
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
            .as_bytes()[self.point.byte_offset..];

        if line_from_offset.is_empty() {
            // We're at the end of the current line, so the next grapheme
            // must be a newline character.
            self.byte_offset += 1;
            self.point.line_idx += 1;
            self.point.byte_offset = 0;
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
            .field(&self.byte_offset)
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

impl From<Point> for api::types::ExtmarkPosition {
    #[inline]
    fn from(point: Point) -> Self {
        Self::ByTuple((point.line_idx, point.byte_offset))
    }
}
