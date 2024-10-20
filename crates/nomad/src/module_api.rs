use fxhash::FxHashMap;
use nvim_oxi::{api, Dictionary};
use serde::de::DeserializeOwned;

use crate::neovim::{Autocmd, DiagnosticMessage};
use crate::{Action, CommandArgs, Module};

pub(super) type OnExecute = Box<
    dyn Fn(api::types::CommandArgs) -> Result<(), DiagnosticMessage> + 'static,
>;

/// TODO: docs.
pub struct ModuleApi<M: Module> {
    module: M,
    dictionary: Dictionary,
    commands: ModuleCommands,
}

pub(super) struct ModuleCommands {
    /// The name of the module these commands belong to.
    pub(super) module_name: &'static str,

    /// The command to run when no command is specified.
    pub(super) default_command: Option<OnExecute>,

    /// Map from command name to the function to run when the command is
    /// executed.
    pub(super) map: FxHashMap<&'static str, OnExecute>,
}

impl<M: Module> ModuleApi<M> {
    #[inline]
    pub fn autocmd<T: Autocmd<M>>(self, autocmd: T) -> Self {
        let _ = autocmd.register();
        self
    }

    #[inline]
    pub fn command<T>(self, command: T) -> Self
    where
        T: Action<Module = M>,
        T::Args: Clone
            + for<'a> TryFrom<
                &'a mut CommandArgs,
                Error: Into<DiagnosticMessage>,
            >,
    {
        self
    }

    #[inline]
    pub fn function<T>(self, function: T) -> Self
    where
        T: Action<Module = M>,
        T::Args: Clone + DeserializeOwned,
    {
        self
    }

    #[inline]
    pub fn new(module: M) -> Self {
        Self {
            module,
            dictionary: Dictionary::default(),
            commands: ModuleCommands::new(M::NAME.as_str()),
        }
    }
}

impl ModuleCommands {
    pub(super) fn default_command(&self) -> Option<&OnExecute> {
        self.default_command.as_ref()
    }

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

    #[track_caller]
    fn add_default_command(&mut self, command: CommandHandle) {
        if self.module_name != command.module_name {
            panic!(
                "trying to register a command for module '{}' in the API for \
                 module '{}'",
                command.module_name, self.module_name
            );
        }

        if self.default_command.is_some() {
            panic!(
                "a default command has already been set for module '{}'",
                self.module_name
            );
        }

        self.default_command = Some(command.on_execute);
    }

    fn new(module_name: &'static str) -> Self {
        Self { module_name, default_command: None, map: FxHashMap::new() }
    }
}
