use super::ModuleId;

/// TODO: docs
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleName {
    name: &'static str,
}

impl core::fmt::Debug for ModuleName {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ModuleName").field(&self.name).finish()
    }
}

impl core::fmt::Display for ModuleName {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.name)
    }
}

impl AsRef<str> for ModuleName {
    #[inline]
    fn as_ref(&self) -> &str {
        self.name
    }
}

impl ModuleName {
    /// TODO: docs
    #[inline]
    pub(crate) fn as_str(&self) -> &'static str {
        self.name
    }

    /// TODO: docs
    #[doc(hidden)]
    pub const fn from_str(name: &'static str) -> Self {
        Self { name }
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn id(&self) -> ModuleId {
        ModuleId::from_module_name(self.name)
    }
}
