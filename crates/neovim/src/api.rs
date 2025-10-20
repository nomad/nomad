//! TODO: docs.

use editor::command::{CommandArgs, CommandCompletion};
use editor::notify::Name;
use editor::{Api, ByteOffset};

use crate::oxi::{Dictionary, Function, Object, api};
use crate::value::NeovimValue;

/// TODO: docs.
pub struct NeovimApi {
    dictionary: Dictionary,
    module_name: Name,
}

impl NeovimApi {
    #[track_caller]
    #[inline]
    fn insert(
        &mut self,
        field_name: &str,
        value: impl Into<Object>,
    ) -> &mut Object {
        if self.dictionary.get(field_name).is_some() {
            panic!(
                "a field with name '{}' has already been added to {}'s API",
                field_name, self.module_name,
            );
        }
        let len = self.dictionary.len();
        self.dictionary.insert(field_name, value.into());
        self.dictionary.as_mut_slice()[len].value_mut()
    }
}

impl Api for NeovimApi {
    type Value = NeovimValue;

    #[inline]
    fn add_command<Cmd, CompFun, Comps>(
        &mut self,
        command_name: Name,
        mut command: Cmd,
        mut completion_fun: CompFun,
    ) where
        Cmd: FnMut(CommandArgs) + 'static,
        CompFun: FnMut(CommandArgs<ByteOffset>) -> Comps + 'static,
        Comps: IntoIterator<Item = CommandCompletion>,
    {
        let command =
            Function::from_fn_mut(move |args: api::types::CommandArgs| {
                command(CommandArgs::<()>::new(
                    args.args.as_deref().unwrap_or_default(),
                ))
            });

        let completion_fun = Function::from_fn_mut(
            move |(_, command_str, mut cursor_offset): (
                String,
                String,
                usize,
            )| {
                let command_str = command_str.as_str();
                // Trim any leading whitespace.
                let initial_len = command_str.len();
                let command_str = command_str.trim_start();
                cursor_offset -= initial_len - command_str.len();

                // The command line must start with "<Command> " for Neovim to
                // invoke us.
                let subcommand_starts_from = command_name.len() + 1;
                debug_assert!(command_str.starts_with(command_name));
                debug_assert!(cursor_offset >= subcommand_starts_from);

                let args = &command_str[subcommand_starts_from..];
                cursor_offset -= subcommand_starts_from;

                completion_fun(CommandArgs::<ByteOffset>::new(
                    args,
                    cursor_offset,
                ))
                .into_iter()
                .map(|comp| comp.as_str().to_owned())
                .collect::<Vec<_>>()
            },
        );

        let opts = api::opts::CreateCommandOpts::builder()
            .complete(api::types::CommandComplete::CustomList(completion_fun))
            .force(true)
            .nargs(api::types::CommandNArgs::Any)
            .build();

        api::create_user_command(command_name, command, &opts)
            .expect("all arguments are valid");
    }

    #[track_caller]
    #[inline]
    fn add_constant(&mut self, constant_name: Name, value: NeovimValue) {
        self.insert(constant_name, value.into_inner());
    }

    #[track_caller]
    #[inline]
    fn add_function<Fun>(&mut self, fun_name: Name, mut fun: Fun)
    where
        Fun: FnMut(NeovimValue) -> Option<NeovimValue> + 'static,
    {
        self.insert(
            fun_name,
            Function::from_fn_mut(move |args| fun(args).unwrap_or_default()),
        );
    }

    #[track_caller]
    #[inline]
    fn add_submodule(&mut self, module_name: Name, module_api: Self) {
        if !module_api.dictionary.is_empty() {
            self.insert(module_name, module_api);
        }
    }

    #[inline]
    fn new(module_name: Name) -> Self {
        Self { dictionary: Dictionary::new(), module_name }
    }
}

impl From<NeovimApi> for Dictionary {
    #[inline]
    fn from(api: NeovimApi) -> Self {
        api.dictionary
    }
}

impl From<NeovimApi> for Object {
    #[inline]
    fn from(api: NeovimApi) -> Self {
        api.dictionary.into()
    }
}
