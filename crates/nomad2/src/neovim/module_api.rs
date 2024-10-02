use std::collections::HashMap;

use nvim_oxi::Dictionary as NvimDictionary;

use super::command::OnExecute;
use super::{CommandHandle, FunctionHandle, Neovim};
use crate::Module;

/// TODO: docs.
pub struct ModuleApi {
    pub(super) name: &'static str,
    pub(super) commands: ModuleCommands,
    pub(super) inner: NvimDictionary,
}

impl ModuleApi {
    /// TODO: docs.
    #[inline]
    pub fn new<M: Module<Neovim>>() -> Self {
        Self {
            name: M::NAME.as_str(),
            commands: ModuleCommands::new(M::NAME.as_str()),
            inner: NvimDictionary::default(),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn with_command(mut self, command: CommandHandle) -> Self {
        self.commands.add_command(command);
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

pub(super) struct ModuleCommands {
    /// The name of the module these commands belong to.
    pub(super) module_name: &'static str,

    /// Map from command name to the function to run when the command is
    /// executed.
    pub(super) map: HashMap<&'static str, OnExecute>,
}

impl ModuleCommands {
    pub(super) fn default_command(&self) -> Option<&OnExecute> {
        todo!();
    }

    fn new(module_name: &'static str) -> Self {
        Self { module_name, map: HashMap::new() }
    }
}

impl ModuleCommands {
    #[track_caller]
    fn add_command(&mut self, command: CommandHandle) {
        if self.module_name != command.module_name {
            panic!(
                "trying to register a command for module '{}' in the API for \
                 module '{}'",
                command.module_name, self.module_name
            );
        }

        if self.map.contains_key(command.name) {
            panic!(
                "a command with the name '{}' already exists in the API for \
                 module '{}'",
                command.name, self.module_name
            );
        }

        self.map.insert(command.name, command.on_execute);
    }
}
