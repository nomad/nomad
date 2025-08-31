use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::ops::Range;

use compact_str::CompactString;
use editor::ByteOffset;
use nvim_oxi::{Array, Integer, Object, String as NvimString, api};

use crate::buffer::Point;
use crate::option::{BufferLocalOpts, NeovimOption, UneditableEndOfLine};

/// An extension trait that adds extra methods to types that can be converted
/// into an [`api::Buffer`].
///
/// Note that all methods in this trait assume that the buffer is valid and
/// loaded (i.e. [`api::Buffer::is_loaded`] returns `true`), and will panic if
/// that's not the case.
pub trait BufferExt {
    /// Returns the buffer that all the methods in this trait will operate on.
    fn buffer(&self) -> api::Buffer;

    /// Returns the number of bytes in the buffer.
    #[track_caller]
    #[inline]
    fn byte_len(&self) -> ByteOffset {
        let buffer = self.buffer();
        let byte_len = buffer
            .line_count()
            .and_then(|line_count| buffer.get_offset(line_count))
            .expect("buffer is valid");
        // Workaround for https://github.com/neovim/neovim/issues/34272.
        if byte_len == 1 && self.has_uneditable_eol() { 0 } else { byte_len }
    }

    /// Converts the given [`Point`] into the corresponding [`ByteOffset`] in
    /// the buffer.
    #[track_caller]
    #[inline]
    fn byte_of_point(&self, point: Point) -> ByteOffset {
        debug_assert!(point <= self.point_of_eof());
        self.num_bytes_before_newline(point.newline_offset) + point.byte_offset
    }

    /// Sets the buffer of the currently focused window to this buffer.
    #[track_caller]
    #[inline]
    fn focus(&self) {
        api::Window::current()
            .set_buf(&self.buffer())
            .expect("couldn't set window buffer");
    }

    /// Whether the [`UneditableEndOfLine`] is enabled for the buffer.
    #[inline]
    fn has_uneditable_eol(&self) -> bool {
        UneditableEndOfLine.get(&BufferLocalOpts::new(self.buffer()))
    }

    /// Returns whether the buffer contains no bytes.
    #[inline]
    fn is_empty(&self) -> bool {
        self.byte_len() == 0
    }

    /// Returns whether the buffer is focused.
    #[inline]
    fn is_focused(&self) -> bool {
        api::Buffer::current() == self.buffer()
    }

    /// Returns whether the given point is after the [`UneditableEndOfLine`].
    #[inline]
    fn is_point_after_uneditable_eol(&self, point: Point) -> bool {
        !self.is_empty()
            && self.has_uneditable_eol()
            && point == self.point_of_eof()
    }

    /// Returns the contents of the first line after the given newline offset,
    /// *without* any trailing newline character.
    ///
    /// Note that if you just want to know the *length* of the line, you should
    /// use [`num_bytes_in_line()`](BufferExt::num_bytes_in_line) instead.
    #[track_caller]
    #[inline]
    fn line_after(&self, newline_offset: usize) -> NvimString {
        let buffer = self.buffer();
        buffer
            .clone()
            .call(move |()| {
                api::call_function(
                    "getbufoneline",
                    (buffer, (newline_offset + 1) as Integer),
                )
            })
            .expect("could not call getbufoneline()")
    }

    /// Returns the text in the given point range.
    #[track_caller]
    #[inline]
    fn get_text_in_point_range(
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
            point_range.end.newline_offset -= 1;
            point_range.end.byte_offset = Integer::MAX as usize;

            // The original start was <= than the end, so if it's now greater
            // it means they were both equal to the point of eof, i.e. the
            // range was empty.
            if point_range.start > point_range.end {
                return CompactString::default();
            }
        }

        let lines = self
            .buffer()
            .get_text(
                point_range.start.newline_offset
                    ..point_range.end.newline_offset,
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

    /// A shorthand for creating a [`GraphemeOffsets`] that starts at the
    /// beginning of the buffer.
    #[inline]
    fn grapheme_offsets(&self) -> GraphemeOffsets<'_> {
        self.grapheme_offsets_from(0)
    }

    /// Returns an iterator over the byte offsets of the grapheme clusters in
    /// the buffer, starting from the given byte offset.
    ///
    /// # Panics
    ///
    /// Panics if the given byte offset is out of bounds or if it doesn't lie
    /// on a character boundary.
    #[inline]
    fn grapheme_offsets_from(
        &self,
        byte_offset: ByteOffset,
    ) -> GraphemeOffsets<'_> {
        debug_assert!(byte_offset <= self.byte_len());
        let point = self.point_of_byte(byte_offset);
        GraphemeOffsets {
            buffer: self.buffer(),
            byte_len: self.byte_len(),
            byte_offset,
            current_line: Some(self.line_after(point.newline_offset)),
            point,
            _not_static: PhantomData,
        }
    }

    /// Returns the buffer's name.
    #[track_caller]
    #[inline]
    fn name(&self) -> NvimString {
        self.buffer().get_name().expect("buffer is valid")
    }

    /// Returns the number of bytes to the left of the given newline offset.
    #[track_caller]
    #[inline]
    fn num_bytes_before_newline(&self, newline_offset: usize) -> ByteOffset {
        debug_assert!(newline_offset <= self.num_newlines());
        // get_offset() already takes care of only counting the final newline
        // if `eol` is enabled.
        self.buffer()
            .get_offset(newline_offset)
            .expect("line index out of bounds")
    }

    /// Same as `self.line_after(newline_offset).len()`, but faster.
    #[track_caller]
    #[inline]
    fn num_bytes_in_line_after(&self, newline_offset: usize) -> ByteOffset {
        debug_assert!(newline_offset <= self.num_newlines());

        // TODO: benchmark whether this is actually faster than
        // `self.line(line_idx).len()`.
        self.buffer()
            .call::<_, _, ByteOffset>(move |()| {
                api::call_function(
                    "col",
                    (Array::from_iter([
                        Object::from((newline_offset + 1) as Integer),
                        Object::from("$"),
                    ]),),
                )
            })
            .expect("could not call col()")
            - 1
    }

    /// Returns the number of newline characters in the buffer.
    #[inline]
    fn num_newlines(&self) -> usize {
        // Workaround for https://github.com/neovim/neovim/issues/34272.
        if self.is_empty() {
            return 0;
        }
        let num_rows = self.buffer().line_count().expect("buffer is valid");
        num_rows - !self.has_uneditable_eol() as usize
    }

    /// Returns the number of newline characters to the left of the given byte
    /// offset.
    #[inline]
    fn num_newlines_before_byte(&self, byte_offset: ByteOffset) -> usize {
        // Fast paths.
        if byte_offset == 0 {
            return 0;
        } else if byte_offset == self.byte_len() {
            return self.num_newlines();
        }

        let buffer = self.buffer();

        buffer
            .clone()
            .call(move |()| {
                let line_idx = api::call_function::<_, usize>(
                    "byte2line",
                    (byte_offset as u32 + 1,),
                ).expect("offset is within bounds")
                // byte2line() returns 1-based line numbers.
                - 1;

                // Whether the character immediately to the left of the given
                // byte offset is a newline.
                let is_offset_after_newline = buffer
                    .get_offset(line_idx + 1)
                    .expect("line index is within bounds")
                    == byte_offset;

                // byte2line() interprets newlines as being the last character
                // of the line they end instead of starting a new one.
                line_idx + is_offset_after_newline as usize
            })
            .expect("could not call function in buffer")
    }

    /// Converts the given [`ByteOffset`] into the corresponding [`Point`] in
    /// the buffer.
    #[track_caller]
    #[inline]
    fn point_of_byte(&self, byte_offset: ByteOffset) -> Point {
        debug_assert!(byte_offset <= self.byte_len());
        let newline_offset = self.num_newlines_before_byte(byte_offset);
        let line_byte_offset = self.num_bytes_before_newline(newline_offset);
        Point::new(newline_offset, byte_offset - line_byte_offset)
    }

    /// Returns the [`Point`] at the end of the buffer.
    ///
    /// This is equivalent to `self.point_of_byte(self.byte_len())`.
    #[inline]
    fn point_of_eof(&self) -> Point {
        let num_newlines = self.num_newlines();
        Point::new(num_newlines, self.num_bytes_in_line_after(num_newlines))
    }

    /// Returns the selected byte range in the buffer, or `None` if the buffer
    /// is not focused or if the user is not in a visual or select mode.
    #[inline]
    fn selection(&self) -> Option<Range<ByteOffset>> {
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

    /// Returns the selected byte range in the buffer, assuming:
    ///
    /// - the buffer is focused;
    /// - the user is in character-wise visual or select mode;
    ///
    /// # Panics
    ///
    /// Panics if either one of those assumptions is not true.
    #[track_caller]
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
            newline_offset: anchor_row - 1,
            byte_offset: anchor_col - 1,
        };

        let head =
            Point { newline_offset: head_row - 1, byte_offset: head_col - 1 };

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
    /// - the buffer is focused;
    /// - the user is in line-wise visual or select mode;
    ///
    /// # Panics
    ///
    /// Panics if either one of those assumptions in not true.
    #[track_caller]
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

        let start_offset = self.num_bytes_before_newline(start_row - 1);

        // Neovim always allows you to select one more character past the end
        // of the line, which is usually interpreted as having selected the
        // following newline.
        //
        // Clearly that doesn't work if you're already at the end of the file.
        let end_offset =
            self.num_bytes_before_newline(end_row).min(self.byte_len());

        start_offset..end_offset
    }
}

impl BufferExt for api::Buffer {
    #[inline]
    fn buffer(&self) -> api::Buffer {
        self.clone()
    }
}

/// TODO: docs.
pub struct GraphemeOffsets<'a> {
    /// The buffer `Self` iterates over.
    buffer: api::Buffer,

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

    /// A lifetime to make sure the iterator doesn't outlive the [`BufferExt`]
    /// it was created from.
    _not_static: PhantomData<&'a ()>,
}

impl Iterator for GraphemeOffsets<'_> {
    type Item = ByteOffset;

    #[track_caller]
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // We're at the end of the buffer.
        if self.byte_offset == self.byte_len {
            return None;
        }

        let line_from_offset = &self
            .current_line
            .get_or_insert_with(|| {
                self.buffer.line_after(self.point.newline_offset)
            })
            .as_bytes()[self.point.byte_offset..];

        if line_from_offset.is_empty() {
            // We're at the end of the current line, so the next grapheme
            // must be a newline character.
            self.byte_offset += 1;
            self.point.newline_offset += 1;
            self.point.byte_offset = 0;
            self.current_line = None;
            Some(self.byte_offset)
        } else {
            // TODO: avoid allocating a new string every time.
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
