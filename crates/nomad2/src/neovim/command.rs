use core::cmp::Ordering;
use core::error::Error;
use core::marker::PhantomData;

use super::Neovim;
use crate::{Context, Emitter, Event, Module, Shared, Subscription};

type OnExecute = Box<dyn Fn(CommandArgs) + 'static>;

/// TODO: docs.
pub fn command<T: Command>(
    ctx: &Context<Neovim>,
) -> (CommandHandle, Subscription<CommandEvent<T>, Neovim>) {
    let buf = Shared::new(None);
    let event = CommandEvent {
        module_name: T::Module::NAME,
        command_name: T::NAME,
        on_execute_buf: buf.clone(),
        ty: PhantomData,
    };
    let sub = ctx.subscribe(event);
    let handle = CommandHandle {
        name: T::NAME,
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
    type Args: TryFrom<CommandArgs>
    where
        <Self::Args as TryFrom<CommandArgs>>::Error: Into<CommandArgsError>;

    /// TODO: docs.
    type Module: Module<Neovim>;
}

/// TODO: docs.
pub struct CommandArgs {}

/// TODO: docs.
pub struct CommandHandle {
    pub(super) name: &'static str,
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
        let on_execute = OnExecute::new(move |args| {
            match T::Args::try_from(args) {
                Ok(payload) => emitter.send(payload),
                Err(err) => {
                    let _err = CommandArgsError::from(err);
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
    fn cmp(&self, other: &Self) -> Ordering {
        match self.module_name.cmp(other.module_name) {
            Ordering::Equal => self.command_name.cmp(other.command_name),
            ord => ord,
        }
    }
}
