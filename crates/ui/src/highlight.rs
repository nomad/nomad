use nvim::api::opts::SetHighlightOpts;
use nvim::api::{self};

use crate::Color;

/// TODO: docs
pub trait Highlight: Sized {
    /// TODO: docs
    const NAME: HighlightName;

    /// TODO: docs
    #[inline]
    fn background(&self) -> Option<Color> {
        None
    }

    /// TODO: docs
    #[inline]
    fn builtin(&self) -> bool {
        false
    }

    /// TODO: docs
    #[inline]
    fn foreground(&self) -> Option<Color> {
        None
    }

    /// TODO: docs
    #[inline]
    fn set(&self, ns_id: u32) {
        if self.builtin() {
            return;
        }

        let mut builder = SetHighlightOpts::builder();

        builder.force(true);

        if let Some(background) = self.background() {
            builder.background(background.as_hex_str());
        }

        if let Some(foreground) = self.foreground() {
            builder.foreground(foreground.as_hex_str());
        }

        api::set_hl(ns_id, Self::NAME.as_str(), &builder.build())
            .expect("both the namespace and the opts are valid");
    }
}

/// TODO: docs
#[derive(Copy, Clone)]
pub struct HighlightName {
    str: &'static str,
}

impl HighlightName {
    /// TODO: docs.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.str
    }

    /// TODO: docs.
    pub const fn new(str: &'static str) -> Self {
        Self { str }
    }
}
