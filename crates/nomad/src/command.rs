use core::convert::Infallible;
use std::collections::HashMap;

use nvim::api::{self, opts, types};
use nvim::Function;

use crate::prelude::{Action, ActionName, Ctx, Module, ModuleId, WarningMsg};
use crate::warning::ChunkExt;

/// TODO: docs
#[derive(Default)]
pub(crate) struct Command {
    map: HashMap<ModuleId, ModuleCommands>,
}

impl Command {
    const NAME: &'static str = "Nomad";

    #[inline]
    pub(crate) fn add_module<M: Module>(&mut self, commands: ModuleCommands) {
        self.map.insert(M::NAME.id(), commands);
    }

    #[inline]
    pub(crate) fn create(self, ctx: Ctx) {
        let opts = opts::CreateCommandOpts::builder()
            .nargs(types::CommandNArgs::OneOrMore)
            .build();

        api::create_user_command(Self::NAME, self.into_func(ctx), &opts)
            .expect("all the arguments are valid");
    }

    #[inline]
    fn into_func(self, ctx: Ctx) -> Function<types::CommandArgs, ()> {
        Function::from_fn(|args: types::CommandArgs| {
            let fargs = args.fargs;
            nvim::print!("{fargs:?}");
            Ok::<_, Infallible>(())
        })
    }
}

/// TODO: docs
#[derive(Default)]
pub(crate) struct ModuleCommands {
    map: HashMap<ActionName, ModuleCommand>,
}

impl ModuleCommands {
    #[inline]
    pub(crate) fn add<M, A>(&mut self, action: A)
    where
        M: Module,
        A: Action<M, Return = ()>,
        A::Args: TryFrom<CommandArgs>,
        <A::Args as TryFrom<CommandArgs>>::Error: Into<WarningMsg>,
    {
        todo!();
    }
}

struct ModuleCommand {}

/// TODO: docs
pub struct CommandArgs {
    args: Vec<String>,
}

impl CommandArgs {
    fn into_iter(self) -> impl Iterator<Item = String> {
        self.args.into_iter()
    }

    fn is_empty(&self) -> bool {
        self.args.is_empty()
    }

    fn len(&self) -> usize {
        self.args.len()
    }
}

impl TryFrom<CommandArgs> for () {
    type Error = CommandArgsNotEmtpy;

    #[inline]
    fn try_from(args: CommandArgs) -> Result<Self, Self::Error> {
        if args.is_empty() {
            Ok(())
        } else {
            Err(CommandArgsNotEmtpy(args))
        }
    }
}

/// An error indicating a command's arguments were not empty.
pub struct CommandArgsNotEmtpy(CommandArgs);

impl From<CommandArgsNotEmtpy> for WarningMsg {
    #[inline]
    fn from(CommandArgsNotEmtpy(args): CommandArgsNotEmtpy) -> WarningMsg {
        assert!(!args.is_empty());

        let mut msg = WarningMsg::new();

        msg.add("expected no arguments, but got ");

        let num_args = args.len();

        for (idx, arg) in args.into_iter().enumerate() {
            msg.add(arg.highlight());

            let is_last = idx + 1 == num_args;

            if is_last {
                break;
            }

            let is_second_to_last = idx + 2 == num_args;

            if is_second_to_last {
                msg.add(" and ");
            } else {
                msg.add(", ");
            }
        }

        msg
    }
}
