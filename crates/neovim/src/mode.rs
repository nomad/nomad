use core::ops::Deref;

/// A newtype around a string slice whose contents are guaranteed to match
/// the textual representation of one of the modes listed under `:help mode()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModeStr<'a>(&'a str);

impl<'a> ModeStr<'a> {
    /// Whether the mode corresponds to a single, contiguous byte range being
    /// selected in a buffer.
    ///
    /// Note that this currently excludes visual|select blockwise mode because
    /// their selections could span several disjoint byte ranges.
    #[inline]
    pub(crate) fn has_selected_range(&self) -> bool {
        self.is_select_by_character()
            || self.is_select_by_line()
            || self.is_visual_by_character()
            || self.is_visual_by_line()
    }

    #[allow(dead_code)]
    #[inline]
    pub(crate) fn is_insert(&self) -> bool {
        self.first_char() == 'i'
    }

    #[allow(dead_code)]
    #[inline]
    pub(crate) fn is_select(&self) -> bool {
        self.is_select_blockwise()
            || self.is_select_by_character()
            || self.is_select_by_line()
    }

    #[allow(dead_code)]
    #[inline]
    pub(crate) fn is_select_blockwise(&self) -> bool {
        self.first_char() == '\u{13}' // CTRL-S
    }

    #[inline]
    pub(crate) fn is_select_by_character(&self) -> bool {
        self.first_char() == 's'
    }

    #[inline]
    pub(crate) fn is_select_by_line(&self) -> bool {
        self.first_char() == 'S'
    }

    #[allow(dead_code)]
    #[inline]
    pub(crate) fn is_visual(&self) -> bool {
        self.is_visual_blockwise()
            || self.is_visual_by_character()
            || self.is_visual_by_line()
    }

    #[inline]
    pub(crate) fn is_visual_blockwise(&self) -> bool {
        self.first_char() == '\u{16}' // CTRL-V
    }

    #[inline]
    pub(crate) fn is_visual_by_character(&self) -> bool {
        self.first_char() == 'v'
    }

    #[inline]
    pub(crate) fn is_visual_by_line(&self) -> bool {
        self.first_char() == 'V'
    }

    #[track_caller]
    #[inline]
    pub(crate) fn new(mode: &'a str) -> Self {
        debug_assert!(!mode.is_empty());
        // FIXME: panic if `mode` is not valid.
        Self(mode)
    }

    #[inline]
    fn first_char(&self) -> char {
        self.as_bytes().first().copied().expect("mode is not empty") as char
    }
}

impl Deref for ModeStr<'_> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}
