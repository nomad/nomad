//! TODO: docs.

use core::fmt;
use core::mem::{self, MaybeUninit};
use core::ops::Deref;

use smallvec::SmallVec;
use smol_str::{SmolStr, ToSmolStr};

use crate::action_ctx::ModulePath;
use crate::backend::BackendExt;
use crate::backend_handle::BackendHandle;
use crate::module::{Module, ModuleName};
use crate::util::OrderedMap;
use crate::{
    Action,
    ActionCtx,
    ActionName,
    Backend,
    ByteOffset,
    MaybeResult,
    NeovimCtx,
    notify,
};

type CommandHandler<B> = Box<dyn FnMut(CommandArgs, &mut ActionCtx<B>)>;

type CommandCompletionFn =
    Box<dyn FnMut(CommandArgs, ByteOffset) -> Vec<CommandCompletion>>;

/// TODO: docs.
pub trait Command<B: Backend>: 'static {
    /// TODO: docs.
    const NAME: &'static ActionName;

    /// TODO: docs.
    type Args: for<'args> TryFrom<CommandArgs<'args>, Error: notify::Error>;

    /// TODO: docs.
    type Docs;

    /// TODO: docs.
    fn call(
        &mut self,
        args: Self::Args,
        ctx: &mut ActionCtx<B>,
    ) -> impl MaybeResult<()>;

    /// TODO: docs.
    fn to_completion_fn(&self) -> impl CompletionFn {
        |_: CommandArgs, _: ByteOffset| core::iter::empty()
    }

    /// TODO: docs.
    fn docs() -> Self::Docs;
}

/// TODO: docs.
pub trait CompletionFn: 'static {
    /// TODO: docs.
    type Completions: IntoIterator<Item = CommandCompletion>;

    /// TODO: docs.
    fn call(
        &mut self,
        args: CommandArgs,
        offset: ByteOffset,
    ) -> Self::Completions;
}

/// TODO: docs.
pub trait ToCompletionFn {
    /// TODO: docs.
    fn to_completion_fn(&self) -> impl CompletionFn;
}

/// TODO: docs.
#[derive(Copy, Clone)]
pub struct CommandArgs<'a> {
    inner: &'a str,
}

/// A group of adjacent non-whitespace characters in a [`CommandArgs`].
#[derive(Copy, Clone)]
pub struct CommandArg<'a> {
    inner: &'a str,
    idx: CommandArgIdx,
}

/// The index of a [`CommandArg`] in a [`CommandArgs`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandArgIdx {
    pub(crate) start: ByteOffset,
    pub(crate) end: ByteOffset,
}

/// An iterator over the [`CommandArg`]s of a [`CommandArgs`].
#[derive(Clone)]
pub struct CommandArgsIter<'a> {
    inner: &'a str,
    last_idx_end: ByteOffset,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone)]
pub enum CommandCursor<'a> {
    /// TODO: docs.
    InArg {
        /// TODO: docs.
        arg: CommandArg<'a>,

        /// TODO: docs.
        offset: ByteOffset,
    },

    /// TODO: docs.
    BetweenArgs {
        /// TODO: docs.
        prev: Option<CommandArg<'a>>,

        /// TODO: docs.
        next: Option<CommandArg<'a>>,
    },
}

/// TODO: docs.
#[derive(Debug, Clone)]
pub struct CommandCompletion {
    kind: CommandCompletionKind,
}

#[derive(Debug, Clone)]
enum CommandCompletionKind {
    Str(SmolStr),
    StaticStr(&'static str),
}

/// TODO: docs.
#[derive(Debug, Copy, Clone)]
pub enum CommandArgsIntoSliceError<'a, T> {
    /// TODO: docs.
    Item(T),

    /// TODO: docs.
    WrongNum(CommandArgsWrongNumError<'a>),
}

/// TODO: docs.
#[derive(Debug, Copy, Clone)]
pub struct CommandArgsWrongNumError<'a> {
    args: CommandArgs<'a>,
    actual_num: usize,
    expected_num: usize,
}

pub(crate) struct CommandBuilder<'a, B> {
    pub(crate) handlers: &'a mut CommandHandlers<B>,
    pub(crate) completions: &'a mut CommandCompletionFns,
}

pub(crate) struct CommandHandlers<B> {
    module_name: &'static ModuleName,
    inner: OrderedMap<&'static str, CommandHandler<B>>,
    submodules: OrderedMap<&'static str, Self>,
}

#[derive(Default)]
pub(crate) struct CommandCompletionFns {
    inner: OrderedMap<&'static str, CommandCompletionFn>,
    submodules: OrderedMap<&'static str, Self>,
}

struct MissingCommandError<'a, B>(&'a CommandHandlers<B>);

struct InvalidCommandError<'a, B>(&'a CommandHandlers<B>, CommandArg<'a>);

impl<'a> CommandArgs<'a> {
    /// TODO: docs.
    #[inline]
    pub fn arg(&self, idx: CommandArgIdx) -> Option<CommandArg<'a>> {
        (self.inner.len() <= idx.end).then_some(CommandArg {
            idx,
            inner: &self.inner[idx.start.into()..idx.end.into()],
        })
    }

    /// TODO: docs.
    #[inline]
    pub fn byte_len(&self) -> ByteOffset {
        self.as_str().len().into()
    }

    /// TODO: docs.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }

    /// TODO: docs.
    #[inline]
    pub fn iter(&self) -> CommandArgsIter<'a> {
        CommandArgsIter { inner: self.as_str(), last_idx_end: 0usize.into() }
    }

    /// TODO: docs.
    #[inline]
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// TODO: docs.
    #[inline]
    pub fn new(args: &'a str) -> Self {
        Self { inner: args }
    }

    /// TODO: docs.
    #[inline]
    pub fn to_cursor(&self, offset: ByteOffset) -> CommandCursor<'a> {
        debug_assert!(offset <= self.inner.len());

        let mut prev = None;
        for arg in self.iter() {
            let idx = arg.idx();
            if offset < idx.start {
                return CommandCursor::BetweenArgs { prev, next: Some(arg) };
            }
            if offset <= idx.end {
                return CommandCursor::InArg {
                    arg,
                    offset: offset - idx.start,
                };
            }
            prev = Some(arg);
        }
        CommandCursor::BetweenArgs { prev, next: None }
    }

    #[inline]
    pub(crate) fn as_str(&self) -> &'a str {
        self.inner
    }

    #[inline]
    pub(crate) fn pop_front(&mut self) -> Option<CommandArg<'a>> {
        let mut iter = self.iter();
        let first = iter.next();
        *self = iter.remainder();
        first
    }
}

impl<'a> CommandArg<'a> {
    /// TODO: docs.
    #[inline]
    pub fn as_str(&self) -> &'a str {
        self.inner
    }

    /// TODO: docs.
    #[inline]
    pub fn end(&self) -> ByteOffset {
        self.idx.end
    }

    /// Returns the index of the argument in the [`CommandArgs`].
    #[inline]
    pub fn idx(&self) -> CommandArgIdx {
        self.idx
    }

    /// TODO: doc.
    #[inline]
    pub fn start(&self) -> ByteOffset {
        self.idx.start
    }
}

impl<'a> CommandArgsIter<'a> {
    #[inline]
    pub(crate) fn remainder(self) -> CommandArgs<'a> {
        CommandArgs { inner: self.inner }
    }
}

impl CommandCompletion {
    /// TODO: docs.
    #[inline]
    pub fn as_str(&self) -> &str {
        match &self.kind {
            CommandCompletionKind::Str(s) => s.as_str(),
            CommandCompletionKind::StaticStr(s) => s,
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn from_static_str(s: &'static str) -> Self {
        Self { kind: CommandCompletionKind::StaticStr(s) }
    }

    /// TODO: docs.
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn from_str(s: &str) -> Self {
        Self { kind: CommandCompletionKind::Str(s.into()) }
    }
}

impl<B> CommandHandlers<B> {
    /// Pushes the list of valid commands and submodules to the given message.
    #[inline]
    fn push_valid(&self, message: &mut notify::Message) {
        let commands = self.inner.keys();
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
}

struct ArgsList<'a>(CommandArgsIter<'a>);

impl fmt::Debug for ArgsList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct DebugAsStr<'a>(CommandArg<'a>);
        impl fmt::Debug for DebugAsStr<'_> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Debug::fmt(self.0.as_ref(), f)
            }
        }

        f.debug_list().entries(self.0.clone().map(DebugAsStr)).finish()
    }
}

impl fmt::Debug for CommandArgs<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("CommandArgs").field(&ArgsList(self.iter())).finish()
    }
}

impl fmt::Debug for CommandArg<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("CommandArg").field(self).finish()
    }
}

impl AsRef<str> for CommandArg<'_> {
    #[inline]
    fn as_ref(&self) -> &str {
        self
    }
}

impl Deref for CommandArg<'_> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl PartialEq<str> for CommandArg<'_> {
    #[inline]
    fn eq(&self, s: &str) -> bool {
        &**self == s
    }
}

impl PartialEq<&str> for CommandArg<'_> {
    #[inline]
    fn eq(&self, s: &&str) -> bool {
        self == *s
    }
}

impl PartialEq<CommandArg<'_>> for str {
    #[inline]
    fn eq(&self, arg: &CommandArg<'_>) -> bool {
        arg == self
    }
}

impl PartialEq<CommandArg<'_>> for &str {
    #[inline]
    fn eq(&self, arg: &CommandArg<'_>) -> bool {
        *self == arg
    }
}

impl fmt::Debug for CommandArgsIter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("CommandArgsIter")
            .field(&ArgsList(self.clone()))
            .finish()
    }
}

impl<'a> Iterator for CommandArgsIter<'a> {
    type Item = CommandArg<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let args = self.inner;
        if args.is_empty() {
            return None;
        }
        let len_whitespace = args.len() - args.trim_start().len();
        let trimmed = &args[len_whitespace..];
        let len_arg = trimmed.find(' ').unwrap_or(trimmed.len());
        let (arg, rest) = trimmed.split_at(len_arg);
        self.inner = rest;
        let idx_start = self.last_idx_end + len_whitespace;
        let idx_end = idx_start + len_arg;
        self.last_idx_end = idx_end;
        (len_arg > 0).then_some(CommandArg {
            inner: arg,
            idx: CommandArgIdx { start: idx_start, end: idx_end },
        })
    }
}

impl<'a> TryFrom<CommandArgs<'a>> for () {
    type Error = CommandArgsWrongNumError<'a>;

    #[inline]
    fn try_from(args: CommandArgs<'a>) -> Result<Self, Self::Error> {
        args.is_empty().then_some(()).ok_or(CommandArgsWrongNumError {
            args,
            actual_num: args.len(),
            expected_num: 0,
        })
    }
}

impl<'a, const N: usize, T> TryFrom<CommandArgs<'a>> for [T; N]
where
    T: TryFrom<CommandArg<'a>>,
{
    type Error = CommandArgsIntoSliceError<'a, T::Error>;

    #[inline]
    fn try_from(args: CommandArgs<'a>) -> Result<Self, Self::Error> {
        let mut array = maybe_uninit_uninit_array::<T, N>();
        let mut num_initialized = 0;
        let mut iter = args.iter();

        let maybe_err = loop {
            let arg = match iter.next() {
                Some(arg) if num_initialized < N => arg,
                Some(_) => {
                    break Some(Self::Error::WrongNum(
                        CommandArgsWrongNumError {
                            args,
                            actual_num: num_initialized + 1 + iter.count(),
                            expected_num: N,
                        },
                    ));
                },
                None if num_initialized < N => {
                    break Some(Self::Error::WrongNum(
                        CommandArgsWrongNumError {
                            args,
                            actual_num: num_initialized,
                            expected_num: N,
                        },
                    ));
                },
                None => break None,
            };
            let item = match T::try_from(arg) {
                Ok(item) => item,
                Err(err) => break Some(Self::Error::Item(err)),
            };
            array[num_initialized] = MaybeUninit::new(item);
            num_initialized += 1;
        };

        if let Some(err) = maybe_err {
            // The initialized elements in the array must be dropped manually.
            for maybe_uninit in &mut array[..num_initialized] {
                // SAFETY: the first `num_initialized` elements have been
                // initialized.
                unsafe { maybe_uninit.assume_init_drop() };
            }
            Err(err)
        } else {
            // SAFETY: MaybeUninit is layout-transparent and all the elements
            // have been initialized.
            Ok(unsafe { maybe_uninit_array_assume_init(array) })
        }
    }
}

impl<'a, B: Backend> CommandBuilder<'a, B> {
    #[inline]
    pub(crate) fn new(
        handlers: &'a mut CommandHandlers<B>,
        completions: &'a mut CommandCompletionFns,
    ) -> Self {
        Self { handlers, completions }
    }

    #[track_caller]
    #[inline]
    pub(super) fn add_command<Cmd>(&mut self, command: Cmd)
    where
        Cmd: Command<B>,
    {
        self.assert_namespace_is_available(Cmd::NAME.as_str());
        self.completions.add_command(&command);
        self.handlers.add_command(command);
    }

    #[track_caller]
    #[inline]
    pub(super) fn add_module<M>(&mut self) -> CommandBuilder<'_, B>
    where
        M: Module<B>,
    {
        self.assert_namespace_is_available(M::NAME.as_str());
        CommandBuilder {
            handlers: self.handlers.add_module::<M>(),
            completions: self.completions.add_module(M::NAME),
        }
    }

    #[track_caller]
    #[inline]
    fn assert_namespace_is_available(&self, namespace: &str) {
        let module_name = self.handlers.module_name.as_str();
        if self.handlers.inner.contains_key(namespace) {
            panic!(
                "a command with name {namespace:?} was already registered on \
                 {module_name:?}'s API",
            );
        }
        if self.completions.inner.contains_key(namespace) {
            panic!(
                "a submodule with name {namespace:?} was already registered \
                 on {module_name:?}'s API",
            );
        }
    }
}

impl<B: Backend> CommandHandlers<B> {
    #[inline]
    pub(crate) fn build(
        mut self,
        backend: BackendHandle<B>,
    ) -> impl FnMut(CommandArgs) + 'static {
        move |args: CommandArgs| {
            backend.with_mut(|backend| {
                let mut module_path = ModulePath::new(self.module_name);
                self.handle(args, &mut module_path, NeovimCtx::new(backend));
            })
        }
    }

    #[inline]
    pub(crate) fn new<M: Module<B>>() -> Self {
        Self {
            module_name: M::NAME,
            inner: Default::default(),
            submodules: Default::default(),
        }
    }

    #[inline]
    fn add_command<Cmd>(&mut self, mut command: Cmd)
    where
        Cmd: Command<B>,
    {
        let handler: CommandHandler<B> = Box::new(move |args, ctx| {
            let args = match Cmd::Args::try_from(args) {
                Ok(args) => args,
                Err(err) => {
                    ctx.emit_action_err(Cmd::NAME, &err);
                    return;
                },
            };
            if let Err(err) = command.call(args, ctx).into_result() {
                ctx.emit_action_err(Cmd::NAME, &err);
            }
        });
        self.inner.insert(Cmd::NAME.as_str(), handler);
    }

    #[inline]
    fn add_module<M: Module<B>>(&mut self) -> &mut Self {
        self.submodules.insert(M::NAME.as_str(), Self::new::<M>())
    }

    #[inline]
    fn handle(
        &mut self,
        mut args: CommandArgs,
        module_path: &mut ModulePath,
        mut ctx: NeovimCtx<B>,
    ) {
        let Some(arg) = args.pop_front() else {
            let err = MissingCommandError(self);
            return ctx.backend_mut().emit_err(module_path, &err);
        };

        if let Some(handler) = self.inner.get_mut(arg.as_str()) {
            let mut action_ctx = ActionCtx::new(ctx, module_path);
            (handler)(args, &mut action_ctx);
        } else if let Some(module) = self.submodules.get_mut(arg.as_str()) {
            module_path.push(module.module_name);
            module.handle(args, module_path, ctx);
        } else {
            let err = InvalidCommandError(self, arg);
            ctx.backend_mut().emit_err(module_path, &err);
        }
    }
}

impl CommandCompletionFns {
    #[inline]
    pub(crate) fn build(
        mut self,
    ) -> impl FnMut(CommandArgs, ByteOffset) -> Vec<CommandCompletion> + 'static
    {
        move |args: CommandArgs, cursor: ByteOffset| {
            self.complete(args, cursor)
        }
    }

    #[inline]
    fn add_command<Cmd, B>(&mut self, command: &Cmd)
    where
        Cmd: Command<B>,
        B: Backend,
    {
        let mut completion_fn = command.to_completion_fn();
        let completion_fn: CommandCompletionFn =
            Box::new(move |args, offset| {
                completion_fn.call(args, offset).into_iter().collect()
            });
        self.inner.insert(Cmd::NAME.as_str(), completion_fn);
    }

    #[inline]
    fn add_module(&mut self, module_name: &'static ModuleName) -> &mut Self {
        self.submodules.insert(module_name.as_str(), Default::default())
    }

    #[inline]
    fn complete(
        &mut self,
        mut args: CommandArgs,
        offset: ByteOffset,
    ) -> Vec<CommandCompletion> {
        debug_assert!(offset <= args.byte_len());

        let Some(arg) = args.pop_front() else {
            return self
                .inner
                .keys()
                .chain(self.submodules.keys())
                .copied()
                .map(CommandCompletion::from_static_str)
                .collect();
        };

        if offset <= arg.end() {
            let prefix = offset
                .checked_sub(arg.start())
                .map(|off| &arg.as_str()[..off.into()])
                .unwrap_or("");

            return self
                .inner
                .keys()
                .chain(self.submodules.keys())
                .filter(|&candidate| candidate.starts_with(prefix))
                .copied()
                .map(CommandCompletion::from_static_str)
                .collect();
        }

        let start_from = arg.end();
        let s = &args.as_str()[start_from.into()..];
        let args = CommandArgs::new(s);
        let offset = offset - start_from;

        if let Some(command) = self.inner.get_mut(arg.as_str()) {
            (command)(args, offset - start_from)
        } else if let Some(submodule) = self.submodules.get_mut(arg.as_str()) {
            submodule.complete(args, offset)
        } else {
            Vec::new()
        }
    }
}

impl<B> notify::Error for MissingCommandError<'_, B> {
    #[inline]
    fn to_level(&self) -> Option<notify::Level> {
        Some(notify::Level::Error)
    }

    #[inline]
    fn to_message(&self) -> notify::Message {
        let Self(handlers) = self;
        let mut message = notify::Message::new();
        let missing = match (
            handlers.inner.is_empty(),
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
        message
    }
}

impl<B> notify::Error for InvalidCommandError<'_, B> {
    #[inline]
    fn to_level(&self) -> Option<notify::Level> {
        Some(notify::Level::Error)
    }

    #[inline]
    fn to_message(&self) -> notify::Message {
        let Self(handlers, arg) = self;
        let mut message = notify::Message::new();
        let invalid = match (
            handlers.inner.is_empty(),
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
            .inner
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

        message
    }
}

impl<A, B> Command<B> for A
where
    A: Action<B, Return = ()> + ToCompletionFn,
    A::Args: for<'args> TryFrom<CommandArgs<'args>, Error: notify::Error>,
    B: Backend,
{
    const NAME: &'static ActionName = A::NAME;

    type Args = A::Args;
    type Docs = A::Docs;

    #[inline]
    fn call(
        &mut self,
        args: Self::Args,
        ctx: &mut ActionCtx<B>,
    ) -> impl MaybeResult<()> {
        A::call(self, args, ctx)
    }

    #[inline]
    fn to_completion_fn(&self) -> impl CompletionFn {
        ToCompletionFn::to_completion_fn(self)
    }

    #[inline]
    fn docs() -> Self::Docs {
        A::docs()
    }
}

impl CompletionFn for () {
    type Completions = core::iter::Empty<CommandCompletion>;

    #[inline]
    fn call(&mut self, _: CommandArgs, _: ByteOffset) -> Self::Completions {
        core::iter::empty()
    }
}

impl<F, R> CompletionFn for F
where
    F: FnMut(CommandArgs, ByteOffset) -> R + 'static,
    R: IntoIterator<Item = CommandCompletion>,
{
    type Completions = R;

    #[inline]
    fn call(
        &mut self,
        args: CommandArgs,
        offset: ByteOffset,
    ) -> Self::Completions {
        (self)(args, offset)
    }
}

impl<T: notify::Error> notify::Error for CommandArgsIntoSliceError<'_, T> {
    #[inline]
    fn to_level(&self) -> Option<notify::Level> {
        match self {
            Self::Item(err) => err.to_level(),
            Self::WrongNum(err) => err.to_level(),
        }
    }

    #[inline]
    fn to_message(&self) -> notify::Message {
        match self {
            Self::Item(err) => err.to_message(),
            Self::WrongNum(err) => err.to_message(),
        }
    }
}

impl notify::Error for CommandArgsWrongNumError<'_> {
    #[inline]
    fn to_level(&self) -> Option<notify::Level> {
        Some(notify::Level::Error)
    }

    #[inline]
    fn to_message(&self) -> notify::Message {
        debug_assert_ne!(self.args.len(), self.expected_num);

        let mut message = notify::Message::new();
        message
            .push_str("expected ")
            .push_expected(self.expected_num.to_smolstr())
            .push_str(" argument")
            .push_str(if self.expected_num == 1 { "" } else { "s" })
            .push_str(", but got ")
            .push_actual(self.actual_num.to_smolstr());

        if !self.args.is_empty() {
            message.push_str(": ").push_comma_separated(
                self.args.iter(),
                notify::SpanKind::Warning,
            );
        }

        message
    }
}

/// Stable version of [`MaybeUninit::uninit_array`].
///
/// Remove this when std's implementation is stabilized.
#[inline]
fn maybe_uninit_uninit_array<T, const N: usize>() -> [MaybeUninit<T>; N] {
    unsafe { mem::MaybeUninit::uninit().assume_init() }
}

/// Stable version of [`MaybeUninit::array_assume_init`].
///
/// Remove this when std's implementation is stabilized.
#[inline]
unsafe fn maybe_uninit_array_assume_init<T, const N: usize>(
    array: [MaybeUninit<T>; N],
) -> [T; N] {
    unsafe { (&array as *const [MaybeUninit<T>; N] as *const [T; N]).read() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_args_iter() {
        let args = CommandArgs::new("  foo bar  baz   ");
        let mut iter = args.iter();
        assert_eq!(iter.next().unwrap(), "foo");
        assert_eq!(iter.next().unwrap(), "bar");
        assert_eq!(iter.next().unwrap(), "baz");
        assert!(iter.next().is_none());
    }
}
