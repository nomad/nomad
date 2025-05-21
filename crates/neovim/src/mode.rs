use core::ops::Deref;

/// A newtype around a string slice whose contents are guaranteed to match
/// the textual representation of one of the modes listed under `:help mode()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModeStr<'a>(&'a str);

impl<'a> ModeStr<'a> {
    #[inline]
    pub(crate) fn new(mode: &'a str) -> Self {
        // FIXME: panic if `mode` is not valid.
        Self(mode)
    }
}

impl Deref for ModeStr<'_> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
