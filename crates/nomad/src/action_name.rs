/// The output of calling [`as_str`](ActionName::as_str) on an [`ActionName`].
pub(crate) type ActionNameStr = &'static str;

/// TODO: docs
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionName {
    name: &'static str,
}

impl core::fmt::Debug for ActionName {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ActionName").field(&self.name).finish()
    }
}

impl core::fmt::Display for ActionName {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.name)
    }
}

impl AsRef<str> for ActionName {
    #[inline]
    fn as_ref(&self) -> &str {
        self.name
    }
}

impl ActionName {
    /// TODO: docs
    #[inline]
    pub(crate) fn as_str(&self) -> ActionNameStr {
        self.name
    }

    #[doc(hidden)]
    pub const fn from_str(name: ActionNameStr) -> Self {
        Self { name }
    }
}
