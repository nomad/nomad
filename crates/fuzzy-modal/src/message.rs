use common::WindowConfig;

use crate::*;

/// TODO: docs
pub enum Message {
    /// TODO: docs
    AddResults(Vec<FuzzyItem>),

    /// TODO: docs
    Close,

    /// TODO: docs
    Confirmed,

    /// TODO: docs
    HidePlaceholder,

    /// TODO: docs
    Open(FuzzyConfig),

    /// TODO: docs
    PromptChanged(PromptDiff),

    /// TODO: docs
    SelectNextItem,

    /// TODO: docs
    SelectPrevItem,

    /// TODO: docs
    ShowPlaceholder,

    /// TODO: docs
    UpdateConfig(Option<WindowConfig>),
}
