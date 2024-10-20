use std::vec::IntoIter;

use crate::neovim::{
    DiagnosticMessage,
    DiagnosticSource,
    HighlightGroup,
    Level,
};

/// TODO: docs.
pub struct CommandArgs {
    inner: IntoIter<String>,
}

/// TODO: docs.
pub(super) struct CommandArgsError {
    source: DiagnosticSource,
    msg: DiagnosticMessage,
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
