use compact_str::CompactString;

/// TODO: docs.
#[derive(Default)]
pub struct Text {
    inner: CompactString,
}

impl Text {
    /// TODO: docs.
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    /// Creates a new empty `Text`.
    pub fn new() -> Self {
        Self::default()
    }

    /// TODO: docs.
    pub fn push(&mut self, ch: char) {
        self.inner.push(ch);
    }

    /// TODO: docs.
    pub fn push_str(&mut self, s: &str) {
        self.inner.push_str(s);
    }
}
