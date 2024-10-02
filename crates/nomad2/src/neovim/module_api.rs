use nvim_oxi::Dictionary as NvimDictionary;

use super::{CommandHandle, FunctionHandle, Neovim};
use crate::Module;

/// TODO: docs.
pub struct ModuleApi {
    pub(super) name: &'static str,
    pub(super) dict: NvimDictionary,
}

impl ModuleApi {
    /// TODO: docs.
    #[inline]
    pub fn new<M: Module<Neovim>>() -> Self {
        Self { name: M::NAME.as_str(), dict: NvimDictionary::default() }
    }

    /// TODO: docs.
    #[inline]
    pub fn with_command(mut self, command: CommandHandle) -> Self {
        todo!();
        // self.dict.insert(command.name, command.inner);
        // self
    }

    /// TODO: docs.
    #[inline]
    pub fn with_function(mut self, function: FunctionHandle) -> Self {
        self.dict.insert(function.name, function.inner);
        self
    }
}
