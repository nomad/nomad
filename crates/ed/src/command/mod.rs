//! TODO: docs.

mod command;
mod command_args;
mod command_builder;

pub use command::{Command, CommandCompletion, CompletionFn, ToCompletionFn};
pub use command_args::{
    CommandArg,
    CommandArgIdx,
    CommandArgs,
    CommandArgsIntoSeqError,
    CommandArgsIter,
    CommandArgsWrongNumError,
    CommandCursor,
    Parse,
};
pub(crate) use command_builder::{CommandBuilder, CommandCompletionsBuilder};
