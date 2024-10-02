use core::cmp::Ordering;
use core::error::Error;
use core::marker::PhantomData;

use super::Neovim;
use crate::{Context, Emitter, Event, Module, Shared, Subscription};

pub(super) type OnExecute = Box<dyn Fn(CommandArgs) + 'static>;

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
            match T::Args::try_from(args).map_err(Into::into) {
                Ok(payload) => emitter.send(payload),
                Err(err) => {
                    todo!();
                },
            };
        });

        self.on_execute_buf.with_mut(|buf| {
            *buf = Some(on_execute);
        });
    }
}

/// TODO: docs.
pub struct CommandArgsError {}

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
