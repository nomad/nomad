use core::cell::RefCell;
use core::convert::Infallible;
use std::collections::hash_map::Values;
use std::collections::HashMap;
use std::rc::Rc;

use nvim::api::{self, opts, types};
use nvim::Function;

use crate::prelude::*;

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
    pub(crate) fn create(self) {
        let opts = opts::CreateCommandOpts::builder()
            .nargs(types::CommandNArgs::OneOrMore)
            .build();

        api::create_user_command(Self::NAME, self.into_func(), &opts)
            .expect("all the arguments are valid");
    }

    #[inline]
    fn into_func(self) -> Function<types::CommandArgs, ()> {
        let Self { map } = self;

        Function::from_fn(move |args: types::CommandArgs| {
            let mut args = CommandArgs::from(args);

            let Some(first) = args.split_first() else {
                unreachable!("Nomad needs OneOrMore arguments")
            };

            let Some(commands) = map.get(&ModuleId::from_module_name(first))
            else {
                Warning::new().msg(UnknownModule(first).into()).print();
                return Ok(());
            };

            let Some(action_name) = args.split_first() else {
                Warning::new()
                    .module(commands.module_name)
                    .msg(MissingAction.into())
                    .print();

                return Ok(());
            };

            match commands.get(action_name) {
                Ok(command) => command.execute(args),

                Err(err) => Warning::new()
                    .module(commands.module_name)
                    .msg(err.into())
                    .print(),
            }

            Ok::<_, Infallible>(())
        })
    }
}

struct UnknownModule<'a>(&'a str);

impl From<UnknownModule<'_>> for WarningMsg {
    #[inline]
    fn from(UnknownModule(name): UnknownModule) -> WarningMsg {
        let mut msg = WarningMsg::new();
        msg.add("unknown module ");
        msg.add(name.highlight());
        msg
    }
}

struct MissingAction;

impl From<MissingAction> for WarningMsg {
    #[inline]
    fn from(_: MissingAction) -> WarningMsg {
        let mut msg = WarningMsg::new();
        msg.add("no action provided");
        msg
    }
}

/// TODO: docs
pub(crate) struct ModuleCommands {
    map: HashMap<ActionId, ModuleCommand>,
    module_name: ModuleName,
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
        self.map.insert(A::NAME.id(), ModuleCommand::new(action));
    }

    #[inline]
    fn get<'this, 'a>(
        &'this self,
        action: &'a str,
    ) -> Result<&'this ModuleCommand, UnknownAction<'a, 'this>> {
        self.map.get(&ActionId::from_action_name(action)).ok_or_else(|| {
            UnknownAction { action, valid_commands: self.map.values() }
        })
    }

    #[inline]
    pub(crate) fn new(module_name: ModuleName) -> Self {
        Self { map: HashMap::new(), module_name }
    }
}

struct UnknownAction<'action, 'values> {
    action: &'action str,
    valid_commands: Values<'values, ActionId, ModuleCommand>,
}

impl From<UnknownAction<'_, '_>> for WarningMsg {
    #[inline]
    fn from(
        UnknownAction { action, valid_commands }: UnknownAction,
    ) -> WarningMsg {
        let mut msg = WarningMsg::new();

        msg.add_invalid(
            action,
            valid_commands.map(|c| c.action_name),
            "action",
        );

        msg
    }
}

struct ModuleCommand {
    action: Box<dyn Fn(CommandArgs)>,
    action_name: ActionName,
}

impl ModuleCommand {
    #[inline]
    fn execute(&self, args: CommandArgs) {
        (self.action)(args);
    }

    #[inline]
    fn new<M, A>(action: A) -> Self
    where
        M: Module,
        A: Action<M, Return = ()>,
        A::Args: TryFrom<CommandArgs>,
        <A::Args as TryFrom<CommandArgs>>::Error: Into<WarningMsg>,
    {
        let action = Rc::new(RefCell::new(action));

        let action = move |args| {
            let action = Rc::clone(&action);

            let future = async move {
                if let Err(err) = exec_action(action, args).await {
                    Warning::new()
                        .module(M::NAME)
                        .action(A::NAME)
                        .msg(err)
                        .print();
                }
            };

            spawn(future).detach();
        };

        Self { action: Box::new(action), action_name: A::NAME }
    }
}

/// TODO: docs
#[allow(clippy::await_holding_refcell_ref)]
#[inline]
async fn exec_action<M, A>(
    action: Rc<RefCell<A>>,
    args: CommandArgs,
) -> Result<(), WarningMsg>
where
    M: Module,
    A: Action<M, Return = ()>,
    A::Args: TryFrom<CommandArgs>,
    <A::Args as TryFrom<CommandArgs>>::Error: Into<WarningMsg>,
{
    let args = A::Args::try_from(args).map_err(Into::into)?;

    let Ok(mut action) = action.try_borrow_mut() else {
        // Should we maybe return an error to notify the user that the
        // action couldn't be executed?
        return Ok(());
    };

    let res = match action.execute(args).into_enum() {
        MaybeFutureEnum::Ready(res) => res,
        MaybeFutureEnum::Future(future) => future.await,
    };

    res.into_result().map_err(Into::into)
}

/// TODO: docs
pub struct CommandArgs {
    /// TODO: docs
    args: Vec<String>,

    /// TODO: docs
    consumed: usize,
}

impl From<types::CommandArgs> for CommandArgs {
    #[inline]
    fn from(args: types::CommandArgs) -> Self {
        Self::new(args.fargs)
    }
}

impl CommandArgs {
    /// TODO: docs
    #[inline]
    pub fn as_slice(&self) -> &[String] {
        &self.args[self.consumed..]
    }

    /// TODO: docs
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// TODO: docs
    #[inline]
    pub fn len(&self) -> usize {
        self.args.len() - self.consumed
    }

    /// TODO: docs
    #[inline]
    fn new(args: Vec<String>) -> Self {
        Self { args, consumed: 0 }
    }

    /// TODO: docs
    #[inline]
    fn split_first(&mut self) -> Option<&str> {
        self.args
            .get(self.consumed)
            .map(String::as_str)
            .inspect(|_| self.consumed += 1)
    }
}

impl IntoIterator for CommandArgs {
    type Item = String;
    type IntoIter = std::iter::Skip<std::vec::IntoIter<Self::Item>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.args.into_iter().skip(self.consumed)
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
