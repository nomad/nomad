use smallvec::SmallVec;

use crate::command::{
    Command,
    CommandArg,
    CommandArgs,
    CommandCompletion,
    CompletionFn,
};
use crate::module::Module;
use crate::notify::{self, MaybeResult, Name, Namespace};
use crate::plugin::{Plugin, PluginId};
use crate::state::{StateHandle, StateMut};
use crate::util::OrderedMap;
use crate::{Borrowed, ByteOffset, Context, Editor};

type CommandHandler<B> =
    Box<dyn FnMut(CommandArgs, &mut Context<B, Borrowed<'_>>)>;

type CommandCompletionFn =
    Box<dyn FnMut(CommandArgs, ByteOffset) -> Vec<CommandCompletion>>;

pub(crate) struct CommandBuilder<Ed: Editor> {
    plugin_id: PluginId,
    /// Map from command name to the handler for that command.
    handlers: OrderedMap<Name, CommandHandler<Ed>>,
    module_name: Name,
    submodules: OrderedMap<Name, Self>,
}

#[derive(Default)]
pub(crate) struct CommandCompletionsBuilder {
    /// Map from command name to the completion function for that command.
    handlers: OrderedMap<Name, CommandCompletionFn>,
    submodules: OrderedMap<Name, Self>,
}

struct MissingCommandError<'a, Ed: Editor>(&'a CommandBuilder<Ed>);

struct InvalidCommandError<'a, Ed: Editor>(
    &'a CommandBuilder<Ed>,
    CommandArg<'a>,
);

impl<Ed: Editor> CommandBuilder<Ed> {
    #[track_caller]
    #[inline]
    pub(crate) fn add_command<Cmd: Command<Ed>>(&mut self, mut command: Cmd) {
        self.assert_namespace_is_available(Cmd::NAME);
        let handler: CommandHandler<Ed> = Box::new(move |args, ctx| {
            let args = match Cmd::Args::try_from(args) {
                Ok(args) => args,
                Err(err) => {
                    ctx.emit_err(err);
                    return;
                },
            };
            if let Err(err) = command.call(args, ctx).into_result() {
                ctx.emit_err(err);
            }
        });
        self.handlers.insert(Cmd::NAME, handler);
    }

    #[track_caller]
    #[inline]
    pub(crate) fn add_module<M: Module<Ed>>(&mut self) -> &mut Self {
        self.assert_namespace_is_available(M::NAME);
        let builder = self.new_for::<M>();
        self.submodules.insert(M::NAME, builder)
    }

    #[inline]
    pub(crate) fn build(
        mut self,
        state: StateHandle<Ed>,
    ) -> impl FnMut(CommandArgs) {
        self.remove_empty_modules();
        move |args: CommandArgs| {
            state.with_mut(|state| {
                let mut namespace = Namespace::new(self.module_name);
                self.handle(args, &mut namespace, state);
            })
        }
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.handlers.is_empty() && self.submodules.is_empty()
    }

    #[inline]
    pub(crate) fn new<P: Plugin<Ed>>() -> Self {
        Self {
            plugin_id: <P as Plugin<_>>::id(),
            module_name: P::NAME,
            handlers: Default::default(),
            submodules: Default::default(),
        }
    }

    #[track_caller]
    #[inline]
    fn assert_namespace_is_available(&self, namespace: &str) {
        let module_name = self.module_name;
        if self.handlers.contains_key(namespace) {
            panic!(
                "a command with name {namespace:?} was already registered on \
                 {module_name:?}'s API",
            );
        }
        if self.submodules.contains_key(namespace) {
            panic!(
                "a submodule with name {namespace:?} was already registered \
                 on {module_name:?}'s API",
            );
        }
    }

    #[inline]
    fn handle(
        &mut self,
        mut args: CommandArgs,
        namespace: &mut Namespace,
        mut state: StateMut<Ed>,
    ) {
        let Some(arg) = args.pop_front() else {
            let err = MissingCommandError(self);
            state.emit_err(namespace, err);
            return;
        };

        if let Some((command_name, handler)) =
            self.handlers.get_key_value_mut(arg.as_str())
        {
            namespace.push(command_name);
            state
                .with_ctx(namespace, self.plugin_id, |ctx| handler(args, ctx));
            namespace.pop();
        } else if let Some(module) = self.submodules.get_mut(arg.as_str()) {
            namespace.push(module.module_name);
            module.handle(args, namespace, state);
        } else {
            let err = InvalidCommandError(self, arg);
            state.emit_err(namespace, err);
        }
    }

    #[inline]
    fn new_for<M: Module<Ed>>(&self) -> Self {
        Self {
            plugin_id: self.plugin_id,
            module_name: M::NAME,
            handlers: Default::default(),
            submodules: Default::default(),
        }
    }

    /// Pushes the list of valid commands and submodules to the given message.
    #[inline]
    fn push_valid(&self, message: &mut notify::Message) {
        let commands = self.handlers.keys();
        let has_commands = commands.len() > 0;
        if has_commands {
            let valid_preface = if commands.len() == 1 {
                "the only valid command is "
            } else {
                "the valid commands are "
            };
            message
                .push_str(valid_preface)
                .push_comma_separated(commands, notify::SpanKind::Expected);
        }

        let submodules = self.submodules.keys();
        if submodules.len() > 0 {
            let valid_preface = if submodules.len() == 1 {
                "the only valid module is "
            } else {
                "the valid modules are "
            };
            message
                .push_str(if has_commands { "; " } else { "" })
                .push_str(valid_preface)
                .push_comma_separated(submodules, notify::SpanKind::Expected);
        }
    }

    /// Recursively removes the modules that don't have any commands in their
    /// subtree.
    #[inline]
    fn remove_empty_modules(&mut self) {
        let mut idx = 0;
        loop {
            let Some((_, builder)) = self.submodules.get_index_mut(idx) else {
                break;
            };
            builder.remove_empty_modules();
            if builder.is_empty() {
                self.submodules.remove_index(idx);
            } else {
                idx += 1;
            }
        }
    }
}

impl CommandCompletionsBuilder {
    #[inline]
    pub(crate) fn add_command<Cmd, Ed>(&mut self, command: &Cmd)
    where
        Cmd: Command<Ed>,
        Ed: Editor,
    {
        let mut completion_fn = command.to_completion_fn();
        let completion_fn: CommandCompletionFn =
            Box::new(move |args, offset| {
                completion_fn.call(args, offset).into_iter().collect()
            });
        self.handlers.insert(Cmd::NAME, completion_fn);
    }

    #[inline]
    pub(crate) fn add_module<M, Ed>(&mut self) -> &mut Self
    where
        M: Module<Ed>,
        Ed: Editor,
    {
        self.submodules.insert(M::NAME, Default::default())
    }

    #[inline]
    pub(crate) fn build(
        mut self,
    ) -> impl FnMut(CommandArgs, ByteOffset) -> Vec<CommandCompletion> + 'static
    {
        self.remove_empty_modules();
        move |args: CommandArgs, cursor: ByteOffset| {
            self.complete(args, cursor)
        }
    }

    #[inline]
    fn complete(
        &mut self,
        mut args: CommandArgs,
        mut offset: ByteOffset,
    ) -> Vec<CommandCompletion> {
        debug_assert!(offset <= args.byte_len());

        let Some(arg) = args.pop_front() else {
            return self
                .handlers
                .keys()
                .chain(self.submodules.keys())
                .copied()
                .map(CommandCompletion::from_static_str)
                .collect();
        };

        if offset <= arg.end() {
            let prefix = offset
                .checked_sub(arg.start())
                .map(|off| &arg.as_str()[..off])
                .unwrap_or("");

            return self
                .handlers
                .keys()
                .chain(self.submodules.keys())
                .filter(|&candidate| candidate.starts_with(prefix))
                .copied()
                .map(CommandCompletion::from_static_str)
                .collect();
        } else {
            offset -= arg.end();
        }

        if let Some(command) = self.handlers.get_mut(arg.as_str()) {
            (command)(args, offset)
        } else if let Some(submodule) = self.submodules.get_mut(arg.as_str()) {
            submodule.complete(args, offset)
        } else {
            Vec::new()
        }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.handlers.is_empty() && self.submodules.is_empty()
    }

    /// Recursively removes the modules that don't have any commands in their
    /// subtree.
    #[inline]
    fn remove_empty_modules(&mut self) {
        let mut idx = 0;
        loop {
            let Some((_, builder)) = self.submodules.get_index_mut(idx) else {
                break;
            };
            builder.remove_empty_modules();
            if builder.is_empty() {
                self.submodules.remove_index(idx);
            } else {
                idx += 1;
            }
        }
    }
}

impl<Ed: Editor> notify::Error for MissingCommandError<'_, Ed> {
    #[inline]
    fn to_message(&self) -> (notify::Level, notify::Message) {
        let Self(handlers) = self;
        let mut message = notify::Message::new();
        let missing = match (
            handlers.handlers.is_empty(),
            handlers.submodules.is_empty(),
        ) {
            (false, false) => "command or submodule",
            (false, true) => "command",
            (true, false) => "submodule",
            (true, true) => unreachable!(),
        };
        message
            .push_str("missing ")
            .push_str(missing)
            .push_str(", ")
            .push_with(|message| handlers.push_valid(message));
        (notify::Level::Error, message)
    }
}

impl<Ed: Editor> notify::Error for InvalidCommandError<'_, Ed> {
    #[inline]
    fn to_message(&self) -> (notify::Level, notify::Message) {
        let Self(handlers, arg) = self;
        let mut message = notify::Message::new();
        let invalid = match (
            handlers.handlers.is_empty(),
            handlers.submodules.is_empty(),
        ) {
            (false, false) => "argument",
            (false, true) => "command",
            (true, false) => "submodule",
            (true, true) => unreachable!(),
        };
        message
            .push_str("invalid ")
            .push_str(invalid)
            .push_str(" ")
            .push_invalid(arg.as_str())
            .push_str(", ");

        let levenshtein_threshold = 2;

        let mut guesses = handlers
            .handlers
            .keys()
            .chain(handlers.submodules.keys())
            .map(|candidate| {
                let distance = strsim::levenshtein(candidate, arg);
                (candidate, distance)
            })
            .filter(|&(_, distance)| distance <= levenshtein_threshold)
            .collect::<SmallVec<[_; 2]>>();

        guesses.sort_by_key(|&(_, distance)| distance);

        if let Some((best_guess, _)) = guesses.first() {
            message
                .push_str("did you mean ")
                .push_expected(best_guess)
                .push_str("?");
        } else {
            handlers.push_valid(&mut message);
        }

        (notify::Level::Error, message)
    }
}
