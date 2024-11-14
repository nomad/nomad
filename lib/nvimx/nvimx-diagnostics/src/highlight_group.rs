use smol_str::SmolStr;

/// TODO: docs.
#[derive(Clone)]
pub struct HighlightGroup(SmolStr);

impl HighlightGroup {
    /// TODO: docs.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// TODO: docs.
    pub fn error() -> Self {
        Self::new("ErrorMsg")
    }

    /// TODO: docs.
    pub fn special() -> Self {
        Self::new("Special")
    }

    /// TODO: docs.
    pub fn warning() -> Self {
        Self::new("WarningMsg")
    }

    pub(super) fn new(group: &str) -> Self {
        Self(SmolStr::new(group))
    }
}

impl From<HighlightGroup> for nvim_oxi::String {
    fn from(group: HighlightGroup) -> Self {
        group.as_str().into()
    }
}
