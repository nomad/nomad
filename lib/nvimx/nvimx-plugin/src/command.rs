use fxhash::FxHashMap;
use nvimx_common::ByteOffset;
use nvimx_common::oxi::{self, api};
use nvimx_diagnostics::{
    DiagnosticMessage,
    DiagnosticSource,
    HighlightGroup,
    Level,
};

use crate::action_name::ActionNameStr;
use crate::module_name::ModuleNameStr;
use crate::module_subcommands::{
    SubCommandCallback,
    SubCommandCompletionFunc,
};
use crate::plugin::Plugin;
use crate::subcommand_args::{
    SubCommandArg,
    SubCommandArgs,
    SubCommandCursor,
};

pub(crate) struct Command {
    inner: CommandInner,
    completion_func: CompletionFunc,
}

#[derive(Default)]
struct CommandInner {
    module_names: Vec<ModuleNameStr>,
    module_subcommands: FxHashMap<ModuleNameStr, ModuleSubCommands>,
}

struct ModuleSubCommands {
    default_subcommand: Option<SubCommandCallback>,
    subcommands: FxHashMap<ActionNameStr, SubCommandCallback>,
    subcommand_names: Vec<ActionNameStr>,
}

struct CompletionFunc {
    command_name: &'static str,
    module_funcs: FxHashMap<ModuleNameStr, ModuleCompletionFunc>,
    module_names: Vec<ModuleNameStr>,
}

struct ModuleCompletionFunc {
    subcommand_funcs: FxHashMap<ActionNameStr, SubCommandCompletionFunc>,
    subcommand_names: Vec<ActionNameStr>,
}

impl Command {
    pub(crate) fn add_module(
        &mut self,
        module_commands: crate::module_subcommands::ModuleSubCommands,
    ) {
        let module_name = module_commands.module_name.as_str();
        assert!(!self.inner.module_subcommands.contains_key(&module_name));

        let subcommand_names = {
            // Sort the subcommand names alphabetically to have nicer error
            // messages.
            let mut v = module_commands
                .subcommands
                .keys()
                .copied()
                .collect::<Vec<_>>();
            v.sort_unstable();
            v
        };
        self.inner.module_subcommands.insert(
            module_name,
            ModuleSubCommands {
                default_subcommand: module_commands.default_subcommand,
                subcommands: module_commands.subcommands,
                subcommand_names: subcommand_names.clone(),
            },
        );
        self.completion_func.module_funcs.insert(
            module_name,
            ModuleCompletionFunc {
                subcommand_names,
                subcommand_funcs: module_commands.completion_funcs,
            },
        );
    }

    pub(crate) fn create(self) {
        let Self { mut inner, mut completion_func } = self;

        let module_names = {
            // Sort the module names alphabetically to have nicer error
            // messages.
            let mut v =
                inner.module_subcommands.keys().copied().collect::<Vec<_>>();
            v.sort_unstable();
            v
        };

        inner.module_names = module_names.clone();
        completion_func.module_names = module_names;

        let command_name = completion_func.command_name;

        let opts = api::opts::CreateCommandOpts::builder()
            .complete(completion_func.into())
            .force(true)
            .nargs(api::types::CommandNArgs::Any)
            .build();

        api::create_user_command(command_name, inner.into_fn(), &opts)
            .expect("all the arguments are valid");
    }

    pub(crate) fn new<P: Plugin>() -> Self {
        Self {
            inner: Default::default(),
            completion_func: CompletionFunc::new(P::COMMAND_NAME),
        }
    }
}

impl CommandInner {
    fn call<'a>(
        &mut self,
        mut args: SubCommandArgs<'a>,
    ) -> Result<(), CommandError<'a>> {
        let Some(module_name) = args.pop_front() else {
            return Err(CommandError::MissingModule {
                valid: self.module_names.clone(),
            });
        };

        let Some(module_subcommands) =
            self.module_subcommands.get_mut(&*module_name)
        else {
            return Err(CommandError::UnknownModule {
                module_name,
                valid: self.module_names.clone(),
            });
        };

        let Some(subcommand_name) = args.pop_front() else {
            return if let Some(default) =
                &mut module_subcommands.default_subcommand
            {
                (default)(args);
                Ok(())
            } else {
                Err(CommandError::MissingSubCommand {
                    module_name,
                    valid: module_subcommands.subcommand_names.clone(),
                })
            };
        };

        match module_subcommands.subcommands.get_mut(&*subcommand_name) {
            Some(subcommand) => {
                (subcommand)(args);
                Ok(())
            },
            None => Err(CommandError::UnknownSubCommand {
                module_name,
                subcommand_name,
                valid: module_subcommands.subcommand_names.clone(),
            }),
        }
    }

    fn into_fn(mut self) -> oxi::Function<api::types::CommandArgs, ()> {
        oxi::Function::from_fn_mut(move |args: api::types::CommandArgs| {
            let args = SubCommandArgs::new(args.args.as_deref().unwrap_or(""));
            if let Err(err) = self.call(args) {
                err.emit()
            }
        })
    }
}

impl CompletionFunc {
    fn complete(
        &mut self,
        args: SubCommandArgs,
        offset: ByteOffset,
    ) -> Vec<String> {
        debug_assert!(offset <= args.as_str().len());

        let mut iter = args.iter();

        let Some(arg) = iter.next() else {
            return self
                .module_names
                .iter()
                .copied()
                .map(ToOwned::to_owned)
                .collect();
        };

        if offset <= arg.idx().end {
            let prefix = offset
                .checked_sub(arg.idx().start)
                .map(|o| &arg[..o.into()])
                .unwrap_or("");
            self.module_names
                .iter()
                .filter(|m| is_strict_prefix(m, prefix))
                .copied()
                .map(ToOwned::to_owned)
                .collect()
        } else {
            match self.module_funcs.get_mut(&*arg) {
                Some(func) => {
                    let start_from = arg.idx().end;
                    let s = &args.as_str()[start_from.into()..];
                    let args = SubCommandArgs::new(s);
                    func.complete(args, offset - start_from)
                },
                None => Vec::new(),
            }
        }
    }

    fn into_fn(
        mut self,
    ) -> oxi::Function<(String, String, usize), Vec<String>> {
        oxi::Function::from_fn_mut(
            move |(_, cmd_line, mut cursor_pos): (String, String, usize)| {
                let initial_len = cmd_line.len();
                let cmd_line = cmd_line.trim_start();
                cursor_pos -= initial_len - cmd_line.len();

                // The command line must start with "<Command> " for Neovim to
                // invoke us.
                let start_from = self.command_name.len() + 1;
                debug_assert!(cmd_line.starts_with(self.command_name));
                debug_assert!(cursor_pos >= start_from);

                let args = SubCommandArgs::new(&cmd_line[start_from..]);
                let offset = ByteOffset::from(cursor_pos - start_from);
                self.complete(args, offset)
            },
        )
    }

    fn new(command_name: &'static str) -> Self {
        Self {
            command_name,
            module_names: Vec::new(),
            module_funcs: FxHashMap::default(),
        }
    }
}

impl ModuleCompletionFunc {
    fn complete(
        &mut self,
        args: SubCommandArgs,
        offset: ByteOffset,
    ) -> Vec<String> {
        debug_assert!(offset <= args.as_str().len());

        let mut iter = args.iter();

        let Some(arg) = iter.next() else {
            return self
                .subcommand_names
                .iter()
                .copied()
                .map(ToOwned::to_owned)
                .collect();
        };

        if offset <= arg.idx().end {
            let prefix = offset
                .checked_sub(arg.idx().start)
                .map(|o| &arg[..o.into()])
                .unwrap_or("");
            self.subcommand_names
                .iter()
                .filter(|&m| is_strict_prefix(m, prefix))
                .copied()
                .map(ToOwned::to_owned)
                .collect()
        } else {
            match self.subcommand_funcs.get_mut(&*arg) {
                Some(sub) => {
                    let start_from = arg.idx().end;
                    let s = &args.as_str()[start_from.into()..];
                    let args = SubCommandArgs::new(s);
                    let offset = offset - start_from;
                    let cursor = SubCommandCursor::new(&args, offset);
                    (sub)(args, cursor)
                },
                None => Vec::new(),
            }
        }
    }
}

fn is_strict_prefix(s: &str, prefix: &str) -> bool {
    s.len() > prefix.len() && s.starts_with(prefix)
}

impl From<CompletionFunc> for api::types::CommandComplete {
    fn from(func: CompletionFunc) -> Self {
        Self::CustomList(func.into_fn())
    }
}

/// The type of error that can occur when [`call`](NomadCommand::call)ing the
/// [`NomadCommand`].
enum CommandError<'args> {
    MissingSubCommand {
        module_name: SubCommandArg<'args>,
        valid: Vec<ActionNameStr>,
    },
    MissingModule {
        valid: Vec<ModuleNameStr>,
    },
    UnknownSubCommand {
        module_name: SubCommandArg<'args>,
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
                source.push_segment(module_name);
            },
            _ => (),
        }
        source
    }
}
