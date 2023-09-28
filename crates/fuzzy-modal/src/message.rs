use common::WindowConfig;

use crate::*;

/// TODO: docs
pub enum Message {
    /// TODO: docs
    AddResults(Vec<FuzzyItem>),

    /// TODO: docs
    Close,

    /// TODO: docs
    Closed,

    /// TODO: docs
    Confirmed,

    /// TODO: docs
    DoneFiltering(u64),

    /// TODO: docs
    HidePlaceholder,

    /// TODO: docs
    Open((FuzzyConfig, ModalId)),

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
