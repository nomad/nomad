/// TODO: docs
#[derive(PartialEq, Eq, Hash)]
pub struct ActionName {
    name: &'static str,
}

impl core::fmt::Debug for ActionName {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ActionName").field("name", &self.name).finish()
    }
}

impl core::fmt::Display for ActionName {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.name)
    }
}

impl ActionName {
    /// TODO: docs
    #[inline]
    pub(crate) fn as_str(&self) -> &'static str {
        self.name
    }

    #[doc(hidden)]
    pub const fn from_str(name: &'static str) -> Self {
        Self { name }
    }
}
