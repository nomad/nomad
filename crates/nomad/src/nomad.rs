use crate::prelude::nvim;
use crate::Module;

/// TODO: docs
#[derive(Default)]
pub struct Nomad {}

impl Nomad {
    /// TODO: docs
    #[inline]
    pub fn api(self) -> nvim::Dictionary {
        nvim::Dictionary::default()
    }

    /// TODO: docs
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// TODO: docs
    #[inline]
    pub fn with_module<M: Module>(self) -> Self {
        self
    }
}
