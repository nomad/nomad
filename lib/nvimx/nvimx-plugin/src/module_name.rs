use core::fmt;

/// The output of calling [`as_str`](crate::ModuleName::as_str) on a
/// [`ModuleName`](crate::ModuleName).
pub(crate) type ModuleNameStr = &'static str;

/// TODO: docs
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleName {
    name: &'static str,
}

impl ModuleName {
    /// TODO: docs
    #[doc(hidden)]
    pub const fn from_str(name: ModuleNameStr) -> Self {
        Self { name }
    }

    /// TODO: docs
    pub const fn as_str(&self) -> ModuleNameStr {
        self.name
    }
}

impl fmt::Debug for ModuleName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("ModuleName").field(&self.name).finish()
    }
}

impl fmt::Display for ModuleName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name)
    }
}

impl AsRef<str> for ModuleName {
    fn as_ref(&self) -> &str {
        self.name
    }
}
