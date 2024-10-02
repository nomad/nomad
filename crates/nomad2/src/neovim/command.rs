use core::cmp::Ordering;
use core::marker::PhantomData;

use super::api::Commands;
use super::module_api::ModuleCommands;
use super::Neovim;
use crate::{Context, Emitter, Event, Module, Shared, Subscription};

pub(super) type OnExecute =
    Box<dyn Fn(CommandArgs) -> Result<(), CommandArgsError> + 'static>;

/// TODO: docs.
pub fn command<T: Command>(
    ctx: &Context<Neovim>,
) -> (CommandHandle, Subscription<CommandEvent<T>, Neovim>) {
    let buf = Shared::new(None);
    let event = CommandEvent {
        module_name: T::Module::NAME.as_str(),
        command_name: T::NAME,
        on_execute_buf: buf.clone(),
        ty: PhantomData,
    };
    let sub = ctx.subscribe(event);
    let handle = CommandHandle {
        name: T::NAME,
        module_name: T::Module::NAME.as_str(),
        on_execute: buf
            .with_mut(Option::take)
            .expect("just set when subscribing"),
    };
    (handle, sub)
}

/// TODO: docs.
pub trait Command: 'static {
    /// TODO: docs.
    const NAME: &'static str;

    /// TODO: docs.
    type Args: TryFrom<CommandArgs, Error: Into<CommandArgsError>>;

    /// TODO: docs.
    type Module: Module<Neovim>;
}

/// TODO: docs.
pub struct CommandArgs {}

impl CommandArgs {
    /// TODO: docs.
    #[inline]
    pub fn pop_front(&mut self) -> Option<String> {
        todo!();
    }
}

impl From<nvim_oxi::api::types::CommandArgs> for CommandArgs {
    #[inline]
    fn from(_: nvim_oxi::api::types::CommandArgs) -> Self {
        todo!();
    }
}

/// TODO: docs.
pub struct CommandHandle {
    pub(super) name: &'static str,
    pub(super) module_name: &'static str,
    pub(super) on_execute: OnExecute,
}

/// TODO: docs.
pub struct CommandEvent<T> {
    module_name: &'static str,
    command_name: &'static str,
    on_execute_buf: Shared<Option<OnExecute>>,
    ty: PhantomData<T>,
}

impl<T: Command> Event<Neovim> for CommandEvent<T> {
    type Payload = T::Args;
    type SubscribeCtx = ();

    #[inline]
    fn subscribe(&mut self, emitter: Emitter<T::Args>, _: &Context<Neovim>) {
        let on_execute = Box::new(move |args| {
            let args = T::Args::try_from(args).map_err(Into::into)?;
            emitter.send(args);
            Ok(())
        });

        self.on_execute_buf.with_mut(|buf| {
            *buf = Some(on_execute);
        });
    }
}

impl<T> PartialEq for CommandEvent<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<T> Eq for CommandEvent<T> {}

impl<T> PartialOrd for CommandEvent<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for CommandEvent<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match self.module_name.cmp(other.module_name) {
            Ordering::Equal => self.command_name.cmp(other.command_name),
            ord => ord,
        }
    }
}

/// TODO: docs.
pub struct CommandArgsError {}

impl CommandArgsError {
    #[inline]
    pub(super) fn missing_command(commands: &ModuleCommands) -> Self {
        todo!();
    }

    #[inline]
    pub(super) fn missing_module(commands: &Commands) -> Self {
        todo!();
    }

    #[inline]
    pub(super) fn unknown_command(
        module_name: &str,
        command: &ModuleCommands,
    ) -> Self {
        todo!();
    }

    #[inline]
    pub(super) fn unknown_module(
        module_name: &str,
        command: &Commands,
    ) -> Self {
        todo!();
    }
}
