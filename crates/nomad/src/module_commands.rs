use fxhash::FxHashMap;

use crate::action_name::ActionNameStr;
use crate::{Command, CommandArgs, Module, ModuleName};

pub(super) struct ModuleCommands {
    /// The name of the module these commands belong to.
    pub(super) module_name: ModuleName,

    /// The command to run when no command is specified.
    pub(super) default_command: Option<Box<dyn FnMut(CommandArgs)>>,

    /// Map from command name to the corresponding [`Command`].
    pub(super) commands: FxHashMap<ActionNameStr, Box<dyn FnMut(CommandArgs)>>,
}

impl ModuleCommands {
    #[track_caller]
    pub(crate) fn add_command<T: Command>(&mut self, command: T) {
        if self.module_name != T::Module::NAME {
            panic!(
                "trying to register a command for module '{}' in the API for \
                 module '{}'",
                T::Module::NAME,
                self.module_name
            );
        }
        if self.commands.contains_key(&T::NAME.as_str()) {
            panic!(
                "a command with the name '{}' already exists in the API for \
                 module '{}'",
                T::NAME,
                self.module_name
            );
        }
        self.commands
            .insert(T::NAME.as_str(), Box::new(command.into_callback()));
    }

    #[track_caller]
    pub(crate) fn add_default_command<T: Command>(&mut self, command: T) {
        if self.module_name != T::Module::NAME {
            panic!(
                "trying to register a command for module '{}' in the API for \
                 module '{}'",
                T::Module::NAME,
                self.module_name
            );
        }

        if self.default_command.is_some() {
            panic!(
                "a default command has already been set for module '{}'",
                self.module_name
            );
        }

        self.default_command = Some(Box::new(command.into_callback()));
    }

    pub(crate) fn default_command(
        &mut self,
    ) -> Option<&mut impl FnMut(CommandArgs)> {
        self.default_command.as_mut()
    }

    pub(crate) fn command<'a>(
        &'a mut self,
        command_name: &'a str,
    ) -> Option<&'a mut impl FnMut(CommandArgs)> {
        self.commands.get_mut(command_name)
    }

    pub(crate) fn command_names(
        &self,
    ) -> impl Iterator<Item = ActionNameStr> + '_ {
        self.commands.keys().copied()
    }

    pub(crate) fn new<M: Module>() -> Self {
        Self {
            module_name: M::NAME,
            default_command: None,
            commands: FxHashMap::default(),
        }
    }
}
