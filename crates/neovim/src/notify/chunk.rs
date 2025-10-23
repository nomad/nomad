use compact_str::CompactString;

/// A chunk of text in a notification message, possibly associated with a given
/// highlight group.
#[derive(Clone)]
pub struct Chunk {
    text: CompactString,
    hl_group: Option<CompactString>,
}

impl Chunk {
    /// Returns the highlight group associated with this chunk, if any.
    #[inline]
    pub fn hl_group(&self) -> Option<&str> {
        self.hl_group.as_deref()
    }

    /// Returns a new chunk with the given text and no highlight group.
    #[inline]
    pub fn new(text: impl Into<CompactString>) -> Self {
        Self { text: text.into(), hl_group: None }
    }

    /// Returns a new chunk highlighted with the given highlight group.
    #[inline]
    pub fn new_highlighted(
        text: impl Into<CompactString>,
        hl_group: impl Into<CompactString>,
    ) -> Self {
        Self { text: text.into(), hl_group: Some(hl_group.into()) }
    }

    /// Returns the text of this chunk.
    #[inline]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[inline]
    pub(crate) fn text_as_compact_str(&self) -> &CompactString {
        &self.text
    }

    #[inline]
    pub(crate) fn text_mut(&mut self) -> &mut CompactString {
        &mut self.text
    }
}
