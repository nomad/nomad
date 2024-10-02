use std::collections::HashMap;

use nvim_oxi::Dictionary as NvimDictionary;

use super::command::OnExecute;
use super::{CommandHandle, FunctionHandle, Neovim};
use crate::Module;

/// TODO: docs.
pub struct ModuleApi {
    pub(super) name: &'static str,
    pub(super) commands: HashMap<&'static str, OnExecute>,
    pub(super) inner: NvimDictionary,
}

impl ModuleApi {
    /// TODO: docs.
    #[inline]
    pub fn new<M: Module<Neovim>>() -> Self {
        Self {
            name: M::NAME.as_str(),
            commands: HashMap::default(),
            inner: NvimDictionary::default(),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn with_command(mut self, command: CommandHandle) -> Self {
        if self.name != command.module_name {
            panic!(
                "trying to register a command for module '{}' in the API for \
                 module '{}'",
                command.module_name, self.name
            );
        }

        if self.commands.contains_key(command.name) {
            panic!(
                "a command with the name '{}' already exists in the API for \
                 module '{}'",
                command.name, self.name
            );
        }

        self.commands.insert(command.name, command.on_execute);
        self
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn with_function(mut self, function: FunctionHandle) -> Self {
        if self.name != function.module_name {
            panic!(
                "trying to register a function for module '{}' in the API \
                 for module '{}'",
                function.module_name, self.name
            );
        }

        if self.inner.get(function.name).is_some() {
            panic!(
                "a function with the name '{}' already exists in the API for \
                 modulle '{}'",
                function.name, self.name
            );
        }

        self.inner.insert(function.name, function.inner);
        self
    }
}
