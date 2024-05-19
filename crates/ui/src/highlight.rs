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
}

/// TODO: docs
#[derive(Copy, Clone)]
pub struct HighlightName {
    str: &'static str,
}

impl core::fmt::Debug for HighlightName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("HighlightName").field(&self.str).finish()
    }
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

/// TODO: docs
pub(crate) struct HighlightGroup {
    name: HighlightName,
    opts: Option<SetHighlightOpts>,
}

impl core::fmt::Debug for HighlightGroup {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("HighlightGroup").field(&self.name.as_str()).finish()
    }
}

impl HighlightGroup {
    /// TODO: docs
    #[inline]
    pub(crate) fn from_highlight<Hl: Highlight>(hl: &Hl) -> Self {
        if hl.builtin() {
            return Self { name: Hl::NAME, opts: None };
        }

        let mut builder = SetHighlightOpts::builder();

        builder.force(true);

        if let Some(background) = hl.background() {
            builder.background(background.as_hex_str());
        }

        if let Some(foreground) = hl.foreground() {
            builder.foreground(foreground.as_hex_str());
        }

        Self { name: Hl::NAME, opts: Some(builder.build()) }
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn name(&self) -> HighlightName {
        self.name
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn set(&self, ns_id: u32) {
        if let Some(opts) = &self.opts {
            api::set_hl(ns_id, self.name.as_str(), opts)
                .expect("both the namespace and the opts are valid");
        }
    }
}

/// TODO: docs
pub struct Normal;

impl Highlight for Normal {
    const NAME: HighlightName = HighlightName::new("Normal");

    #[inline]
    fn builtin(&self) -> bool {
        true
    }
}
