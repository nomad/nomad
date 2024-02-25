/// TODO: docs
pub struct ModuleName {
    name: &'static str,
}

impl ModuleName {
    #[doc(hidden)]
    pub const fn from_str(name: &'static str) -> Self {
        Self { name }
    }
}
