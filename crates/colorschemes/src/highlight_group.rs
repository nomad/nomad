use common::nvim::api::opts::{SetHighlightOpts, SetHighlightOptsBuilder};

use crate::Color;

/// TODO: docs
#[derive(Default)]
pub struct HighlightGroup {
    builder: SetHighlightOptsBuilder,
}

impl HighlightGroup {
    pub fn background(mut self, color: Color) -> Self {
        self.builder.background(color.as_hex_string().as_str());
        self
    }

    pub fn foreground(mut self, color: Color) -> Self {
        self.builder.foreground(color.as_hex_string().as_str());
        self
    }

    pub fn link(mut self, link: &'static str) -> Self {
        self.builder.link(link);
        self
    }

    pub fn new() -> Self {
        Self::default()
    }
}

impl From<HighlightGroup> for SetHighlightOpts {
    fn from(mut group: HighlightGroup) -> Self {
        group.builder.build()
    }
}
