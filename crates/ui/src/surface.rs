use core::ops::{Range, RangeBounds};

use api::types::*;
use nvim::api;

use crate::{Bound, Cells, HighlightGroup, Point};

pub(crate) type ByteOffset = usize;

/// TODO: docs
pub(crate) struct Surface {
    /// TODO: docs.
    buffer: api::Buffer,

    /// TODO: docs.
    window: api::Window,

    /// TODO: docs.
    namespace: u32,
}

impl Surface {
    /// A helper function that panics with a message indicating that the buffer
    /// was removed.
    ///
    /// This can be used when a method on the [`Surface`]'s buffer fails if the
    /// caller can guarantee that all the arguments passed where valid, in
    /// which case the method must've failed because the buffer was removed by
    /// the user.
    fn buf_was_removed(&self) -> ! {
        panic!("{:?} was removed", self.buffer)
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn hide(&mut self) {
        let config = WindowConfig::builder().hide(true).build();
        let _ = self.window.set_config(&config);
    }

    /// TODO: docs
    #[inline]
    fn highlight_line_range<R>(
        &mut self,
        line: usize,
        range: R,
        hl: &HighlightGroup,
    ) where
        R: RangeBounds<ByteOffset>,
    {
        hl.set(self.namespace);

        let _ = self.buffer.add_highlight(
            self.namespace,
            hl.name().as_str(),
            line,
            range,
        );
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn highlight_text(
        &mut self,
        range: Range<Point<ByteOffset>>,
        hl: &HighlightGroup,
    ) {
        let start = range.start;

        let end = range.end;

        if start.y() == end.y() {
            self.highlight_line_range(start.y(), start.x()..end.x(), hl);
            return;
        }

        let mut line_range = start.y()..=end.y();

        let Some(first_line) = line_range.next() else { return };

        self.highlight_line_range(first_line, start.x().., hl);

        let Some(last_line) = line_range.next_back() else { return };

        self.highlight_line_range(last_line, ..end.x(), hl);

        for line in line_range {
            self.highlight_line_range(line, .., hl);
        }
    }

    /// Returns the numbef of lines in the buffer.
    ///
    /// Note that Neovim considers an empty buffer to have one line, so the
    /// return value is always greater than or equal to `1`.
    #[inline]
    fn line_count(&self) -> usize {
        self.buffer.line_count().unwrap_or_else(|_| self.buf_was_removed())
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.line_count() == 1
            && self.lines_inner().next().expect("len always >= 1").is_empty()
    }

    #[inline]
    pub(crate) fn is_hidden(&self) -> bool {
        self.window
            .get_config()
            .map(|config| config.hide.unwrap_or(false))
            .unwrap_or(false)
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn _lines(&self) -> impl ExactSizeIterator<Item = String> {
        self.lines_inner()
            .take((!self.is_empty() as usize) * self.line_count())
    }

    #[inline]
    fn lines_inner(&self) -> impl ExactSizeIterator<Item = String> {
        self.buffer
            .get_lines(.., true)
            .unwrap_or_else(|_| self.buf_was_removed())
            .map(|nvim_string| nvim_string.to_string_lossy().into_owned())
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn new_hidden() -> Self {
        let buffer = api::create_buf(false, true).expect("never fails(?)");

        let config = WindowConfig::builder()
            .relative(WindowRelativeTo::Editor)
            .height(1)
            .width(1)
            .row(0)
            .col(0)
            .hide(true)
            .style(WindowStyle::Minimal)
            .build();

        let window = api::open_win(&buffer, false, &config)
            .expect("the config is valid");

        let namespace = api::create_namespace("nomad-ui");

        Self { buffer, window, namespace }
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn delete_lines(
        &mut self,
        line_range: impl RangeBounds<usize>,
    ) {
        let _ = self.buffer.set_lines(
            line_range,
            true,
            core::iter::empty::<nvim::String>(),
        );
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn insert_lines(
        &mut self,
        line_offset: usize,
        mut lines: impl Iterator<Item = impl Into<nvim::String>>,
    ) {
        // If the buffer is empty, the first line is used to replace the empty
        // line.
        if line_offset == 0 && self.is_empty() {
            let Some(first_line) = lines.next() else { return };
            self.replace_text(0, 0..0, first_line.into());
            self.insert_lines(line_offset + 1, lines);
            return;
        }

        let _ = self.buffer.set_lines(line_offset..line_offset, true, lines);
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn replace_text(
        &mut self,
        line_idx: usize,
        byte_range: Range<ByteOffset>,
        text: impl Into<nvim::String>,
    ) {
        let _ = self.buffer.set_text(
            line_idx..line_idx,
            byte_range.start,
            byte_range.end,
            [text],
        );
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn resize_window(&mut self, new_size: Bound<Cells>) {
        let config = WindowConfig::builder()
            .height(new_size.height().into())
            .width(new_size.width().into())
            .build();

        let _ = self.window.set_config(&config);
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn show(&mut self) {
        let config = WindowConfig::builder().hide(false).build();
        let _ = self.window.set_config(&config);
    }
}
