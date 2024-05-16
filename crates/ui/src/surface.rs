use core::ops::{Range, RangeBounds};

use api::types::*;
use nvim::api;

use crate::{Bound, Cells, Highlight};

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
    /// TODO: docs
    #[inline]
    pub(crate) fn highlight_line_range<R, Hl>(
        &mut self,
        line: usize,
        range: R,
        hl: &Hl,
    ) where
        R: RangeBounds<ByteOffset>,
        Hl: Highlight,
    {
        hl.set(self.namespace);

        let _ = self.buffer.add_highlight(
            self.namespace,
            <Hl as Highlight>::NAME.as_str(),
            line,
            range,
        );
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn highlight_text<Hl: Highlight>(
        &mut self,
        range: Range<Point>,
        hl: &Hl,
    ) {
        let start = range.start;

        let end = range.end;

        if start.line == end.line {
            self.highlight_line_range(
                start.line,
                start.offset..end.offset,
                hl,
            );

            return;
        }

        let mut line_range = start.line..=end.line;

        let Some(first_line) = line_range.next() else { return };

        self.highlight_line_range(first_line, start.offset.., hl);

        let Some(last_line) = line_range.next_back() else { return };

        self.highlight_line_range(last_line, ..end.offset, hl);

        for line in line_range {
            self.highlight_line_range(line, .., hl);
        }
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
    fn replace_text(&mut self, range: Range<Point>, text: &str) {
        let lines = text.lines().chain(text.ends_with('\n').then_some(""));

        let _ = self.buffer.set_text(
            range.start.line..range.end.line,
            range.start.offset,
            range.end.offset,
            lines,
        );
    }

    /// TODO: docs
    #[inline]
    fn resize_window(&mut self, new_size: Bound<Cells>) {
        let config = WindowConfig::builder()
            .height(new_size.height().into())
            .width(new_size.width().into())
            .build();

        let _ = self.window.set_config(&config);
    }
}

/// TODO: docs
pub(crate) struct Point {
    line: ByteOffset,
    offset: ByteOffset,
}

impl Point {
    #[inline]
    pub(crate) fn new(line: ByteOffset, offset: ByteOffset) -> Self {
        Self { line, offset }
    }
}
