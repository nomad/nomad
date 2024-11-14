use crate::highlight_group::HighlightGroup;

/// TODO: docs.
pub enum Level {
    /// TODO: docs.
    Warning,

    /// TODO: docs.
    Error,
}

impl From<Level> for HighlightGroup {
    fn from(level: Level) -> Self {
        match level {
            Level::Warning => HighlightGroup::warning(),
            Level::Error => HighlightGroup::error(),
        }
    }
}
