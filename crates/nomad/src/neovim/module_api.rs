use std::collections::HashMap;

use nvim_oxi::Dictionary as NvimDictionary;

use super::command::OnExecute;
use super::config::{ConfigEvent, OnConfigChange};
use super::{CommandHandle, FunctionHandle, Neovim};
use crate::{Context, Module, Shared, Subscription};

/// TODO: docs.
pub fn module_api<M: Module<Neovim>>(
    ctx: &Context<Neovim>,
) -> (ModuleApi, Subscription<ConfigEvent<M>, Neovim>) {
    let buf = Shared::new(None);
    let event = ConfigEvent::<M>::new(buf.clone());
    let sub = ctx.subscribe(event);
    let api = ModuleApi {
        name: M::NAME.as_str(),
        commands: ModuleCommands::new(M::NAME.as_str()),
        on_config_change: buf
            .with_mut(Option::take)
            .expect("just set when subscribing"),
        inner: NvimDictionary::default(),
    };
    (api, sub)
}

/// TODO: docs.
pub struct ModuleApi {
    pub(super) name: &'static str,
    pub(super) commands: ModuleCommands,
    pub(super) inner: NvimDictionary,
    pub(super) on_config_change: OnConfigChange,
}

impl ModuleApi {
    /// TODO: docs.
    #[track_caller]
    pub fn with_command(mut self, command: CommandHandle) -> Self {
        self.commands.add_command(command);
        self
    }

    /// TODO: docs.
    #[track_caller]
    pub fn with_default_command(mut self, command: CommandHandle) -> Self {
        self.commands.add_default_command(command);
        self
    }

    /// TODO: docs.
    #[track_caller]
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

    /// The command to run when no command is specified.
    pub(super) default_command: Option<OnExecute>,

    /// Map from command name to the function to run when the command is
    /// executed.
    pub(super) map: HashMap<&'static str, OnExecute>,
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
        Self { module_name, default_command: None, map: HashMap::new() }
    }
}
