use core::cmp::Ordering;
use core::marker::PhantomData;
use std::vec::IntoIter;

use super::api::Commands;
use super::diagnostic::{
    DiagnosticMessage,
    DiagnosticSource,
    HighlightGroup,
    Level,
};
use super::module_api::ModuleCommands;
use super::Neovim;
use crate::{Context, Emitter, Event, Module, Shared, Subscription};

pub(super) type OnExecute =
    Box<dyn Fn(CommandArgs) -> Result<(), DiagnosticMessage> + 'static>;

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
    type Args: for<'a> TryFrom<
        &'a mut CommandArgs,
        Error: Into<DiagnosticMessage>,
    >;

    /// TODO: docs.
    type Module: Module<Neovim>;
}

/// TODO: docs.
pub struct CommandArgs {
    inner: IntoIter<String>,
}

impl CommandArgs {
    /// TODO: docs.
    pub fn as_slice(&self) -> &[String] {
        self.inner.as_slice()
    }

    /// TODO: docs.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// TODO: docs.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// TODO: docs.
    pub fn pop_front(&mut self) -> Option<String> {
        self.inner.next()
    }
}

impl<T: TryFrom<String>> TryFrom<&mut CommandArgs> for Vec<T> {
    type Error = T::Error;

    fn try_from(args: &mut CommandArgs) -> Result<Self, Self::Error> {
        let mut buf = Vec::with_capacity(args.len());
        while let Some(arg) = args.pop_front() {
            let item = T::try_from(arg)?;
            buf.push(item);
        }
        Ok(buf)
    }
}

impl<T1, T2> TryFrom<&mut CommandArgs> for (T1, T2)
where
    T1: for<'a> TryFrom<&'a mut CommandArgs, Error: Into<DiagnosticMessage>>,
    T2: for<'a> TryFrom<&'a mut CommandArgs, Error: Into<DiagnosticMessage>>,
{
    type Error = DiagnosticMessage;

    fn try_from(args: &mut CommandArgs) -> Result<Self, Self::Error> {
        let t1 = T1::try_from(args).map_err(Into::into)?;
        let t2 = T2::try_from(args).map_err(Into::into)?;
        Ok((t1, t2))
    }
}

impl From<nvim_oxi::api::types::CommandArgs> for CommandArgs {
    fn from(args: nvim_oxi::api::types::CommandArgs) -> Self {
        Self { inner: args.fargs.into_iter() }
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

    fn subscribe(&mut self, emitter: Emitter<T::Args>, _: &Context<Neovim>) {
        let on_execute = Box::new(move |mut args| {
            let args = T::Args::try_from(&mut args).map_err(Into::into)?;
            emitter.send(args);
            Ok(())
        });

        self.on_execute_buf.with_mut(|buf| {
            *buf = Some(on_execute);
        });
    }
}

impl<T> PartialEq for CommandEvent<T> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<T> Eq for CommandEvent<T> {}

impl<T> PartialOrd for CommandEvent<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for CommandEvent<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.module_name.cmp(other.module_name) {
            Ordering::Equal => self.command_name.cmp(other.command_name),
            ord => ord,
        }
    }
}

/// TODO: docs.
pub(super) struct CommandArgsError {
    source: DiagnosticSource,
    msg: DiagnosticMessage,
}

impl CommandArgsError {
    pub(super) fn emit(self) {
        self.msg.emit(Level::Error, self.source);
    }

    pub(super) fn missing_command(commands: &ModuleCommands) -> Self {
        debug_assert!(!commands.map.is_empty());

        let mut source = DiagnosticSource::new();
        source.push_segment(commands.module_name);

        let mut msg = DiagnosticMessage::new();
        msg.push_str("missing command, the valid commands are ")
            .push_comma_separated(
                commands.map.keys(),
                HighlightGroup::special(),
            );

        Self { source, msg }
    }

    pub(super) fn missing_module(commands: &Commands) -> Self {
        debug_assert!(!commands.map.is_empty());

        let mut msg = DiagnosticMessage::new();
        msg.push_str("missing module, the valid modules are ")
            .push_comma_separated(
                commands.map.keys(),
                HighlightGroup::special(),
            );

        Self { source: DiagnosticSource::new(), msg }
    }

    pub(super) fn new(
        source: DiagnosticSource,
        msg: DiagnosticMessage,
    ) -> Self {
        Self { source, msg }
    }

    pub(super) fn unknown_command(
        command_name: &str,
        commands: &ModuleCommands,
    ) -> Self {
        debug_assert!(!commands.map.is_empty());

        let mut source = DiagnosticSource::new();
        source.push_segment(commands.module_name);

        let mut msg = DiagnosticMessage::new();
        msg.push_str("unknown command '")
            .push_str_highlighted(command_name, HighlightGroup::special())
            .push_str("', the valid commands are ")
            .push_comma_separated(
                commands.map.keys(),
                HighlightGroup::special(),
            );

        Self { source, msg }
    }

    pub(super) fn unknown_module(
        module_name: &str,
        commands: &Commands,
    ) -> Self {
        debug_assert!(!commands.map.is_empty());

        let mut msg = DiagnosticMessage::new();
        msg.push_str("unknown module '")
            .push_str_highlighted(module_name, HighlightGroup::special())
            .push_str("', the valid modules are ")
            .push_comma_separated(
                commands.map.keys(),
                HighlightGroup::special(),
            );

        Self { source: DiagnosticSource::new(), msg }
    }
}

impl TryFrom<&mut CommandArgs> for () {
    type Error = DiagnosticMessage;

    fn try_from(args: &mut CommandArgs) -> Result<Self, Self::Error> {
        if args.is_empty() {
            Ok(())
        } else {
            let mut msg = DiagnosticMessage::new();
            msg.push_str("unexpected arguments: ").push_comma_separated(
                args.as_slice(),
                HighlightGroup::special(),
            );
            Err(msg)
        }
    }
}
