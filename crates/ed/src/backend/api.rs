//! TODO: docs.

use crate::ByteOffset;
use crate::backend::Value;
use crate::command::{CommandArgs, CommandCompletion};
use crate::notify::Name;

/// TODO: docs.
pub trait Api: 'static + Sized {
    /// TODO: docs.
    type Value: Value;

    /// TODO: docs.
    fn add_constant(&mut self, constant_name: Name, value: Self::Value);

    /// TODO: docs.
    fn add_function<Fun>(&mut self, function_name: Name, function: Fun)
    where
        Fun: FnMut(Self::Value) -> Option<Self::Value> + 'static;

    /// TODO: docs.
    fn add_submodule(&mut self, module_name: Name, module_api: Self);

    /// TODO: docs.
    fn add_command<Command, CompletionFn, Completions>(
        &mut self,
        command_name: Name,
        command: Command,
        completion_fn: CompletionFn,
    ) where
        Command: FnMut(CommandArgs) + 'static,
        CompletionFn: FnMut(CommandArgs, ByteOffset) -> Completions + 'static,
        Completions: IntoIterator<Item = CommandCompletion>;

    /// TODO: docs.
    fn new(module_name: Name) -> Self;
}
