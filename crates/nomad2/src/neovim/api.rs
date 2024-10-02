use core::ops::AddAssign;
use std::collections::HashMap;

use nvim_oxi::lua::ffi::State as LuaState;
use nvim_oxi::{
    api,
    lua,
    Dictionary as NvimDictionary,
    Function as NvimFunction,
    Object as NvimObject,
};

use super::module_api::ModuleCommands;
use super::{CommandArgs, CommandArgsError, ModuleApi, Neovim};
use crate::Nomad;

/// TODO: docs.
const NOMAD_CMD_NAME: &str = "Mad";

/// TODO: docs.
const SETUP_FN_NAME: &str = "setup";

/// TODO: docs.
#[derive(Default)]
pub struct Api {
    commands: Commands,
    dict: NvimDictionary,
}

impl AddAssign<ModuleApi> for Api {
    #[track_caller]
    fn add_assign(&mut self, module_api: ModuleApi) {
        if self.dict.get(&module_api.name).is_some() {
            panic!(
                "a module with the name '{}' has already been added to the \
                 API",
                module_api.name
            );
        }

        if module_api.name == SETUP_FN_NAME {
            panic!(
                "got a module with the name '{}', which is reserved for the \
                 setup function",
                module_api.name
            );
        }

        self.dict.insert(module_api.name, module_api.inner);
    }
}

fn setup(_obj: NvimObject) {}

#[derive(Default)]
pub(super) struct Commands {
    /// Map from module name to the commands for that module.
    pub(super) map: HashMap<&'static str, ModuleCommands>,
}

impl Commands {
    fn create_mad_command(self) {
        let opts = api::opts::CreateCommandOpts::builder()
            .nargs(api::types::CommandNArgs::Any)
            .build();

        api::create_user_command(NOMAD_CMD_NAME, self.on_execute(), &opts)
            .expect("all the arguments are valid");
    }

    fn on_execute(self) -> impl Fn(api::types::CommandArgs) + 'static {
        move |args| {
            if let Err(err) = self.on_execute_inner(args) {
                todo!();
            }
        }
    }

    fn on_execute_inner(
        &self,
        args: api::types::CommandArgs,
    ) -> Result<(), CommandArgsError> {
        let mut args = CommandArgs::from(args);

        let module_name = args
            .pop_front()
            .ok_or_else(|| CommandArgsError::missing_module(self))?;

        let module_commands =
            self.map.get(&module_name.as_str()).ok_or_else(|| {
                CommandArgsError::unknown_module(&module_name, self)
            })?;

        let Some(command_name) = args.pop_front() else {
            return (if let Some(default) = module_commands.default_command() {
                default(args)
            } else {
                Err(CommandArgsError::missing_command(module_commands))
            });
        };

        let command = module_commands
            .map
            .get(&command_name.as_str())
            .ok_or_else(|| {
                CommandArgsError::unknown_command(
                    &command_name,
                    module_commands,
                )
            })?;

        command(args)
    }
}

impl AddAssign<ModuleCommands> for Commands {
    #[track_caller]
    fn add_assign(&mut self, commands: ModuleCommands) {
        if self.map.contains_key(&commands.module_name) {
            panic!(
                "a module with the name '{}' has already been added to the \
                 API",
                commands.module_name
            );
        }

        self.map.insert(commands.module_name, commands);
    }
}

impl lua::Pushable for Api {
    unsafe fn push(mut self, state: *mut LuaState) -> Result<i32, lua::Error> {
        self.commands.create_mad_command();

        let setup = NvimFunction::from_fn(|obj| setup(obj));
        self.dict.insert(SETUP_FN_NAME, setup);
        self.dict.push(state)
    }
}

impl lua::Pushable for Nomad<Neovim> {
    unsafe fn push(mut self, state: *mut LuaState) -> Result<i32, lua::Error> {
        self.start_modules();
        self.into_api().push(state)
    }
}
