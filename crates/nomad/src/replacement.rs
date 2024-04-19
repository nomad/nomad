use core::ops::Range;

use smol_str::SmolStr;

/// An replacement edit on a buffer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Replacement<Offset> {
    start: Offset,
    end: Offset,
    replacement: SmolStr,
}

impl<Offset: Copy> Replacement<Offset> {
    /// TODO: docs.
    #[inline]
    pub fn deletion(range: Range<Offset>) -> Self {
        Self::new(range, SmolStr::default())
    }

    /// The end of the replaced range.
    #[inline]
    pub fn end(&self) -> Offset {
        self.end
    }

    /// TODO: docs.
    #[inline]
    pub fn insertion(at: Offset, text: impl Into<SmolStr>) -> Self {
        Self::new(at..at, text.into())
    }

    /// TODO: docs.
    #[inline]
    pub fn new(range: Range<Offset>, replacement: impl Into<SmolStr>) -> Self {
        Self {
            start: range.start,
            end: range.end,
            replacement: replacement.into(),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn no_op() -> Self
    where
        Offset: Default,
    {
        Self::insertion(Default::default(), "")
    }

    /// The deleted range.
    #[inline]
    pub fn range(&self) -> Range<Offset> {
        self.start..self.end
    }

    /// The text the range is replaced with.
    #[inline]
    pub fn replacement(&self) -> &str {
        &self.replacement
    }

    /// The end of the replaced range.
    #[inline]
    pub fn start(&self) -> Offset {
        self.start
    }
}
