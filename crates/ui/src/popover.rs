use crate::{Render, Window};

/// TODO: docs
pub struct Popover {
    /// TODO: docs
    anchor: PopoverAnchor,

    /// TODO: docs
    root: Box<dyn Render + 'static>,

    /// TODO: docs
    window: Window,
}

/// TODO: docs
pub enum PopoverAnchor {}
