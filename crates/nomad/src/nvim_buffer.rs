use core::ops::{Bound, Range, RangeBounds};

use nvim::api::{self, opts};
use smol_str::SmolStr;

use crate::{ByteOffset, Edit, Point, Replacement, Shared};

type OnEdit = Box<dyn FnMut(&Replacement<ByteOffset>) + 'static>;

/// A handle to a Neovim buffer.
#[cfg_attr(not(feature = "tests"), doc(hidden))]
#[derive(Clone)]
pub struct NvimBuffer {
    /// The buffer handle.
    inner: api::Buffer,

    /// The list of callbacks to be called every time the buffer is edited.
    on_edit_callbacks: Shared<Vec<OnEdit>>,
}

impl core::fmt::Debug for NvimBuffer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("NvimBuffer").field(&self.inner).finish()
    }
}

impl NvimBuffer {
    #[inline]
    fn attach(buf: api::Buffer) -> Result<Self, NvimBufferDoesntExistError> {
        let on_edit_callbacks = Shared::<Vec<OnEdit>>::default();

        let cbs = on_edit_callbacks.clone();

        let opts = opts::BufAttachOpts::builder()
            .on_bytes(move |args| {
                let edit = Replacement::from(args);
                cbs.with_mut(|cbs| cbs.iter_mut().for_each(|cb| cb(&edit)));
                Ok(false)
            })
            .build();

        buf.attach(false, &opts)
            // All the arguments passed to `attach()` are valid, so if it fails
            // it must be because the buffer doesn't exist.
            .map_err(|_| NvimBufferDoesntExistError)?;

        Ok(Self::new(buf, on_edit_callbacks))
    }

    /// Creates a new buffer.
    #[inline]
    pub fn create() -> Self {
        let Ok(buf) = api::create_buf(true, false) else { unreachable!() };
        let Ok(buf) = Self::attach(buf) else { unreachable!() };
        buf
    }

    /// Creates a new buffer.
    #[inline]
    pub fn current() -> Self {
        let buf = api::Buffer::current();
        let Ok(buf) = Self::attach(buf) else { unreachable!() };
        buf
    }

    /// TODO: docs
    #[inline]
    pub fn delete(
        &mut self,
        range: Range<Point<ByteOffset>>,
    ) -> Result<(), api::Error> {
        self.replace(range, "")
    }

    /// Edits the buffer.
    #[inline]
    pub fn edit<E>(&mut self, edit: E) -> E::Diff
    where
        E: Edit<Self>,
    {
        edit.apply(self)
    }

    /// Returns the [`Point`] at the end of the buffer.
    #[inline]
    fn end_point(&self) -> Result<Point<ByteOffset>, api::Error> {
        let num_lines = self.inner.line_count()?;

        if num_lines == 0 {
            return Ok(Point::default());
        }

        let last_line_len = self.inner.get_offset(num_lines)?
            // Calling `get_offset()` with the number of lines always seem to
            // include the trailing newline, even when `eol` is turned off.
            - 1
            - self.inner.get_offset(num_lines - 1)?;

        let point = Point::new(num_lines - 1, ByteOffset::new(last_line_len));

        Ok(point)
    }

    /// TODO: docs.
    #[inline]
    pub fn get<R>(&self, range: R) -> Result<String, api::Error>
    where
        R: RangeBounds<Point<ByteOffset>>,
    {
        let start = match range.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start,
            Bound::Unbounded => Point::default(),
        };

        let end = match range.end_bound() {
            Bound::Included(&end) => end,
            Bound::Excluded(&end) => end,
            Bound::Unbounded => self.end_point()?,
        };

        let mut lines = self.inner.get_text(
            start.line()..end.line(),
            start.offset().into(),
            end.offset().into(),
            &Default::default(),
        )?;

        let mut text = String::new();

        let Some(first_line) = lines.next() else {
            return Ok(text);
        };

        text.push_str(&first_line.to_string_lossy());

        for line in lines {
            text.push('\n');
            text.push_str(&line.to_string_lossy());
        }

        Ok(text)
    }

    pub(crate) fn inner(&self) -> &api::Buffer {
        &self.inner
    }

    pub(crate) fn inner_mut(&mut self) -> &mut api::Buffer {
        &mut self.inner
    }

    /// TODO: docs
    #[inline]
    pub fn insert(
        &mut self,
        insert_at: Point<ByteOffset>,
        replacement: &str,
    ) -> Result<(), api::Error> {
        self.replace(insert_at..insert_at, replacement)
    }

    /// Registers a callback to be called every time the buffer is edited.
    #[inline]
    pub fn on_edit<F: FnMut(&Replacement<ByteOffset>) + 'static>(
        &self,
        callback: F,
    ) {
        self.on_edit_callbacks
            .with_mut(|callbacks| callbacks.push(Box::new(callback)));
    }

    #[inline]
    fn new(buf: api::Buffer, on_edit_callbacks: Shared<Vec<OnEdit>>) -> Self {
        Self { inner: buf, on_edit_callbacks }
    }

    /// TODO: docs
    #[inline]
    fn replace(
        &mut self,
        range: Range<Point<ByteOffset>>,
        text: &str,
    ) -> Result<(), api::Error> {
        // If the text has a trailing newline, the iterator we feed to
        // `set_text` has to yield a final empty line for it to work like we
        // want it to.
        let lines = text.lines().chain(text.ends_with('\n').then_some(""));

        self.inner.set_text(
            range.start.line()..range.end.line(),
            range.start.offset().into(),
            range.end.offset().into(),
            lines,
        )
    }
}

impl TryFrom<&NvimBuffer> for crop::Rope {
    type Error = api::Error;

    #[inline]
    fn try_from(buf: &NvimBuffer) -> Result<Self, Self::Error> {
        let buf = &buf.inner;

        let num_lines = buf.line_count()?;

        let has_trailine_newline = {
            let mut last_line =
                buf.get_lines(num_lines - 1..num_lines, true)?;

            let last_line_len =
                buf.get_offset(num_lines)? - buf.get_offset(num_lines - 1)?;

            if let Some(last_line) = last_line.next() {
                last_line_len > last_line.len()
            } else {
                false
            }
        };

        let lines = buf.get_lines(0..num_lines, true)?;

        let mut builder = crop::RopeBuilder::new();

        for (idx, line) in lines.enumerate() {
            builder.append(line.to_string_lossy());
            let is_last = idx + 1 == num_lines;
            let should_append_newline = !is_last | has_trailine_newline;
            if should_append_newline {
                builder.append("\n");
            }
        }

        Ok(builder.build())
    }
}

impl Edit<NvimBuffer> for &Replacement<Point<ByteOffset>> {
    type Diff = ();

    #[inline]
    fn apply(self, buf: &mut NvimBuffer) -> Self::Diff {
        if let Err(err) = buf.replace(self.range(), self.replacement()) {
            panic!("couldn't apply replacement: {err}");
        }
    }
}

impl Edit<NvimBuffer> for Replacement<Point<ByteOffset>> {
    type Diff = ();

    #[inline]
    fn apply(self, buf: &mut NvimBuffer) -> Self::Diff {
        (&self).apply(buf)
    }
}

impl From<opts::OnBytesArgs> for Replacement<ByteOffset> {
    #[inline]
    fn from(
        (
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
        ): opts::OnBytesArgs,
    ) -> Self {
        let buf = NvimBuffer::new(buf, Shared::default());

        let start = Point::new(start_row, start_col.into());

        let end = Point::new(
            start_row + new_end_row,
            (start_col * (new_end_row == 0) as usize + new_end_col).into(),
        );

        let replacement = if start == end {
            SmolStr::default()
        } else {
            buf.get(start..end)
                .expect("buffer exists and range is valid")
                .into()
        };

        let replaced_range =
            start_offset.into()..(start_offset + old_end_len).into();

        Self::new(replaced_range, replacement)
    }
}

/// An error returned whenever a..
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NvimBufferDoesntExistError;
