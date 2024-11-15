use std::vec::IntoIter;

use nvim_oxi::api::types;

use crate::diagnostics::{DiagnosticMessage, HighlightGroup};

/// TODO: docs.
#[derive(Debug, Clone)]
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

impl From<types::CommandArgs> for CommandArgs {
    fn from(args: types::CommandArgs) -> Self {
        Self { inner: args.fargs.into_iter() }
    }
}

impl<'a> TryFrom<&'a mut CommandArgs> for () {
    type Error = CommandArgsWrongNumError<'a>;

    fn try_from(args: &'a mut CommandArgs) -> Result<Self, Self::Error> {
        args.is_empty()
            .then_some(())
            .ok_or(CommandArgsWrongNumError { args, expected_num: 0 })
    }
}

impl<'a, const N: usize> TryFrom<&'a mut CommandArgs> for &'a [String; N] {
    type Error = CommandArgsWrongNumError<'a>;

    fn try_from(args: &'a mut CommandArgs) -> Result<Self, Self::Error> {
        args.as_slice()
            .try_into()
            .ok()
            .ok_or(CommandArgsWrongNumError { args, expected_num: N })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct CommandArgsWrongNumError<'a> {
    args: &'a CommandArgs,
    expected_num: usize,
}

impl From<CommandArgsWrongNumError<'_>> for DiagnosticMessage {
    fn from(err: CommandArgsWrongNumError) -> Self {
        assert_ne!(err.args.len(), err.expected_num);

        let mut message = DiagnosticMessage::new();
        message
            .push_str("expected ")
            .push_str_highlighted(
                err.expected_num.to_string(),
                HighlightGroup::special(),
            )
            .push_str(" argument")
            .push_str(if err.expected_num == 1 { "" } else { "s" })
            .push_str(", but got ")
            .push_str_highlighted(
                err.args.len().to_string(),
                HighlightGroup::warning(),
            );

        if !err.args.is_empty() {
            message.push_str(": ").push_comma_separated(
                err.args.as_slice(),
                HighlightGroup::warning(),
            );
        }

        message
    }
}
