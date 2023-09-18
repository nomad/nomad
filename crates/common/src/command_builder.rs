use std::convert::Infallible;

use crate::nvim::{self, Function};
use crate::nvim::{api::opts::*, api::types::CommandArgs};
use crate::sender::Sender;
use crate::Plugin;

/// TODO: docs
pub struct CommandBuilder<'a, P: Plugin> {
    sender: &'a Sender<P::Message>,
}

struct Command {
    name: &'static str,
    func: Option<Function<CommandArgs, ()>>,
    opts: Option<CreateCommandOptsBuilder>,
}

impl<'a, P: Plugin> CommandBuilder<'a, P> {
    /// TODO: docs
    pub fn command(
        &mut self,
        name: &'static str,
    ) -> OnExecuteCommandBuilder<'a, '_, P> {
        let command = Command { name, func: None, opts: None };
        OnExecuteCommandBuilder { command, builder: self }
    }

    /// TODO: docs
    pub fn new(sender: &'a Sender<P::Message>) -> Self {
        Self { sender }
    }
}

pub struct OnExecuteCommandBuilder<'a, 'builder, P: Plugin> {
    command: Command,
    builder: &'builder mut CommandBuilder<'a, P>,
}

impl<'a, 'builder, P: Plugin> OnExecuteCommandBuilder<'a, 'builder, P> {
    /// TODO: docs
    pub fn on_execute<F>(mut self, func: F) -> OptsCommandBuilder
    where
        F: Fn(CommandArgs) -> P::Message + 'static,
    {
        let sender = self.builder.sender.clone();
        let func = move |args| {
            let msg = func(args);
            sender.send(msg);
            Ok::<_, Infallible>(())
        };
        let func = Function::from_fn(func);
        self.command.func = Some(func);
        OptsCommandBuilder { command: self.command }
    }
}

pub struct OptsCommandBuilder {
    command: Command,
}

impl OptsCommandBuilder {
    /// TODO: docs
    pub fn build(self) {
        build(self.command);
    }

    fn opts_builder(&mut self) -> &mut CreateCommandOptsBuilder {
        self.command.opts.get_or_insert_with(CreateCommandOpts::builder)
    }

    /// TODO: docs
    pub fn with_desc(mut self, desc: &'static str) -> Self {
        self.opts_builder().desc(desc);
        self
    }
}

fn build(command: Command) {
    let Some(func) = command.func else {
        return;
    };

    let opts =
        command.opts.map(|mut builder| builder.build()).unwrap_or_default();

    nvim::api::create_user_command(command.name, func, &opts).unwrap();
}
