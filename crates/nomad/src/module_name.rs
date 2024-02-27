/// TODO: docs
#[derive(PartialEq, Eq, Hash)]
pub struct ModuleName {
    name: &'static str,
}

impl core::fmt::Debug for ModuleName {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ModuleName").field("name", &self.name).finish()
    }
}

impl core::fmt::Display for ModuleName {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.name)
    }
}

impl ModuleName {
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
