use fxhash::FxHashMap;
use nvimx_common::oxi::{self, api};
use nvimx_common::ByteOffset;
use nvimx_diagnostics::{
    DiagnosticMessage,
    DiagnosticSource,
    HighlightGroup,
    Level,
};

use crate::action_name::ActionNameStr;
use crate::module_name::{ModuleName, ModuleNameStr};
use crate::module_subcommands::ModuleSubCommands;
use crate::plugin::Plugin;
use crate::subcommand_args::{
    SubCommandArg,
    SubCommandArgs,
    SubCommandCursor,
};

pub(crate) struct Command {
    command_name: &'static str,
    /// A map from module name to the subcommands for that module.
    subcommands: FxHashMap<ModuleNameStr, ModuleSubCommands>,
}

impl Command {
    pub(crate) fn add_module(&mut self, module_commands: ModuleSubCommands) {
        let module_name = module_commands.module_name.as_str();
        if self.subcommands.contains_key(&module_name) {
            panic!(
                "subcommands from a module named '{}' have already been added",
                module_name
            );
        }
        self.subcommands.insert(module_name, module_commands);
    }

    pub(crate) fn create(mut self) {
        let opts = api::opts::CreateCommandOpts::builder()
            .nargs(api::types::CommandNArgs::Any)
            .complete(api::types::CommandComplete::CustomList(
                self.completion_func(),
            ))
            .build();

        api::create_user_command(
            self.command_name,
            move |args: api::types::CommandArgs| {
                let args =
                    SubCommandArgs::new(args.args.as_deref().unwrap_or(""));
                if let Err(err) = self.call(args) {
                    err.emit()
                }
            },
            &opts,
        )
        .expect("all the arguments are valid");
    }

    pub(crate) fn new<P: Plugin>() -> Self {
        Self {
            command_name: P::COMMAND_NAME,
            subcommands: FxHashMap::default(),
        }
    }

    fn call<'a>(
        &mut self,
        mut args: SubCommandArgs<'a>,
    ) -> Result<(), CommandError<'a>> {
        let Some(module_name) = args.pop_front() else {
            return Err(CommandError::MissingModule {
                valid: self.subcommands.keys().copied().collect(),
            });
        };

        let Some(module_subcommands) = self.subcommands.get_mut(&*module_name)
        else {
            return Err(CommandError::UnknownModule {
                module_name,
                valid: self.subcommands.keys().copied().collect(),
            });
        };

        let Some(subcommand_name) = args.pop_front() else {
            return if let Some(default) =
                module_subcommands.default_subcommand()
            {
                default.call(args);
                Ok(())
            } else {
                Err(CommandError::MissingSubCommand {
                    module_name: module_subcommands.module_name,
                    valid: module_subcommands.names().collect(),
                })
            };
        };

        match module_subcommands.subcommand(&subcommand_name) {
            Some(subcommand) => {
                subcommand.call(args);
                Ok(())
            },
            None => Err(CommandError::UnknownSubCommand {
                module_name: module_subcommands.module_name,
                subcommand_name,
                valid: module_subcommands.names().collect(),
            }),
        }
    }

    fn completion_func(
        &self,
    ) -> oxi::Function<(String, String, usize), Vec<String>> {
        let command_name = self.command_name;
        let module_names = {
            let mut v = self.subcommands.keys().copied().collect::<Vec<_>>();
            v.sort_unstable();
            v
        };
        let mut this = clone();
        let func = move |(_, cmd_line, cursor_pos): (_, String, usize)| {
            use CursorCompletePos::*;
            let cmd_line = cmd_line.trim_start();

            // The command line must start with "<Command> " for Neovim to
            // invoke us.
            let start_from = command_name.len() + 1;
            debug_assert!(cmd_line.starts_with(&command_name));
            debug_assert!(cursor_pos >= start_from);
            let args = SubCommandArgs::new(&cmd_line[start_from..]);
            let offset = ByteOffset::from(cursor_pos - start_from);

            let names = module_names.iter().copied().map(ToOwned::to_owned);
            let subcommands: &mut ModuleSubCommands = match pos() {
                NoArgs { .. } | StartOfArg { .. } | BeforeArg { .. } => {
                    return names.collect()
                },
                EndOfArg { arg } => {
                    let arg = &*arg;
                    return names
                        .filter(|m| m.len() > arg.len() && m.starts_with(arg))
                        .collect();
                },
                MiddleOfArg { arg, offset_in_arg } => {
                    let arg = &arg[..offset_in_arg.into()];
                    return names
                        .filter(|m| m.len() > arg.len() && m.starts_with(arg))
                        .collect();
                },
                AfterArg { arg } => match this.subcommands.get_mut(&*arg) {
                    Some(subs) => subs,
                    None => return Vec::new(),
                },
            };

            let names = subcommands.names().map(ToOwned::to_owned);
            match pos() {
                NoArgs { .. } | StartOfArg { .. } | BeforeArg { .. } => {
                    names.collect()
                },
                EndOfArg { arg } => {
                    let arg = &*arg;
                    names
                        .filter(|m| m.len() > arg.len() && m.starts_with(arg))
                        .collect()
                },
                MiddleOfArg { arg, offset_in_arg } => {
                    let arg = &arg[..offset_in_arg.into()];
                    names
                        .filter(|m| m.len() > arg.len() && m.starts_with(arg))
                        .collect()
                },
                AfterArg { arg } => {
                    drop(names);
                    match subcommands.subcommand(&*arg) {
                        Some(sub) => {
                            let cursor = SubCommandCursor::new(&args, offset);
                            sub.complete(args, cursor)
                        },
                        None => Vec::new(),
                    }
                },
            }
        };
        oxi::Function::from_fn_mut(func)
    }
}

fn clone() -> Command {
    todo!();
}

fn pos() -> CursorCompletePos<'static> {
    CursorCompletePos::NoArgs
}

enum CursorCompletePos<'a> {
    /// `|`.
    NoArgs,

    /// `|<Arg>`.
    StartOfArg { arg: SubCommandArg<'a> },

    /// `<Arg>|`.
    EndOfArg { arg: SubCommandArg<'a> },

    /// `<Ar|g>`.
    MiddleOfArg { arg: SubCommandArg<'a>, offset_in_arg: ByteOffset },

    /// `| <Arg>`.
    ///
    /// `Arg` is interpreted as a subcommand name, and we return the modules
    /// that have a subcommand with that name.
    BeforeArg { arg: SubCommandArg<'a> },

    /// `<Arg> |`.
    AfterArg { arg: SubCommandArg<'a> },
}

/// The type of error that can occur when [`call`](NomadCommand::call)ing the
/// [`NomadCommand`].
enum CommandError<'args> {
    MissingSubCommand {
        module_name: ModuleName,
        valid: Vec<ActionNameStr>,
    },
    MissingModule {
        valid: Vec<ModuleNameStr>,
    },
    UnknownSubCommand {
        module_name: ModuleName,
        subcommand_name: SubCommandArg<'args>,
        valid: Vec<ActionNameStr>,
    },
    UnknownModule {
        module_name: SubCommandArg<'args>,
        valid: Vec<ModuleNameStr>,
    },
}

impl CommandError<'_> {
    fn emit(self) {
        self.message().emit(Level::Warning, self.source());
    }

    fn message(&self) -> DiagnosticMessage {
        let mut message = DiagnosticMessage::new();
        match self {
            Self::MissingSubCommand { valid, .. } => {
                message
                    .push_str(
                        "missing subcommand, the valid subcommands are: ",
                    )
                    .push_comma_separated(valid, HighlightGroup::special());
            },
            Self::MissingModule { valid } => {
                message
                    .push_str("missing module, the valid modules are: ")
                    .push_comma_separated(valid, HighlightGroup::special());
            },

            Self::UnknownSubCommand { subcommand_name, valid, .. } => {
                message
                    .push_str("unknown subcommand '")
                    .push_str_highlighted(
                        subcommand_name,
                        HighlightGroup::warning(),
                    )
                    .push_str("', the valid subcommands are: ")
                    .push_comma_separated(valid, HighlightGroup::special());
            },
            Self::UnknownModule { module_name, valid } => {
                message
                    .push_str("unknown module '")
                    .push_str_highlighted(
                        module_name,
                        HighlightGroup::warning(),
                    )
                    .push_str("', the valid modules are: ")
                    .push_comma_separated(valid, HighlightGroup::special());
            },
        }
        message
    }

    fn source(&self) -> DiagnosticSource {
        let mut source = DiagnosticSource::new();
        match self {
            Self::UnknownSubCommand { module_name, .. }
            | Self::MissingSubCommand { module_name, .. } => {
                source.push_segment(module_name.as_str());
            },
            _ => (),
        }
        source
    }
}
