use core::marker::PhantomData;
use std::vec::IntoIter;

use super::api::Commands;
use super::diagnostic::{
    DiagnosticMessage,
    DiagnosticSource,
    HighlightGroup,
    Level,
};
use super::events::CommandEvent;
use super::module_api::ModuleCommands;
use super::Neovim;
use crate::{Context, Module, Shared, Subscription};

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
    type Args: Clone
        + for<'a> TryFrom<&'a mut CommandArgs, Error: Into<DiagnosticMessage>>;

    /// TODO: docs.
    type Module: Module<Neovim>;
}

/// TODO: docs.
pub struct CommandHandle {
    pub(super) name: &'static str,
    pub(super) module_name: &'static str,
    pub(super) on_execute: OnExecute,
}
