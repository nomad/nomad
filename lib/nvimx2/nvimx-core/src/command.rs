//! TODO: docs.

use core::borrow::Borrow;

use smol_str::SmolStr;

use crate::backend::BackendExt;
use crate::backend_handle::BackendHandle;
use crate::module::{Module, ModuleName};
use crate::{
    Action,
    ActionName,
    Backend,
    ByteOffset,
    MaybeResult,
    NeovimCtx,
    notify,
};

type CommandHandler<B> =
    Box<dyn FnMut(CommandArgs, &mut notify::Namespace, NeovimCtx<B>)>;

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
        ctx: NeovimCtx<'_, B>,
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
pub struct CommandArgs<'a>(&'a str);

/// TODO: docs.
pub struct CommandArg<'a>(&'a str);

/// TODO: docs.
pub struct CommandCompletion {
    inner: SmolStr,
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

struct OrderedMap<K, V> {
    inner: Vec<(K, V)>,
}

struct MissingCommandError<'a, B>(&'a CommandHandlers<B>);

struct UnknownCommandError<'a, B>(&'a CommandHandlers<B>, CommandArg<'a>);

impl<'a> CommandArgs<'a> {
    /// TODO: docs.
    #[inline]
    pub fn as_str(&self) -> &'a str {
        self.0
    }

    /// TODO: docs.
    #[inline]
    pub fn byte_len(&self) -> ByteOffset {
        ByteOffset::from(self.0.len())
    }

    /// TODO: docs.
    #[inline]
    pub fn new(_command_str: &'a str) -> Self {
        todo!()
    }

    /// TODO: docs.
    #[inline]
    pub fn next(&mut self) -> Option<CommandArg<'a>> {
        todo!()
    }
}

impl<'a> CommandArg<'a> {
    /// TODO: docs.
    #[inline]
    pub fn as_str(&self) -> &'a str {
        todo!()
    }

    /// TODO: docs.
    #[inline]
    pub fn end(&self) -> ByteOffset {
        todo!()
    }

    /// TODO: docs.
    #[inline]
    pub fn start(&self) -> ByteOffset {
        todo!()
    }
}

impl CommandCompletion {
    /// TODO: docs.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    /// TODO: docs.
    #[inline]
    pub fn from_static_str(s: &'static str) -> Self {
        todo!()
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
                let mut namespace = notify::Namespace::default();
                self.handle(args, &mut namespace, NeovimCtx::new(backend));
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
        let handler: CommandHandler<B> =
            Box::new(move |args, namespace, mut ctx| {
                namespace.set_action(Cmd::NAME);
                let args = match Cmd::Args::try_from(args) {
                    Ok(args) => args,
                    Err(err) => {
                        ctx.backend_mut().emit_err(namespace, &err);
                        return;
                    },
                };
                if let Err(err) =
                    command.call(args, ctx.as_mut()).into_result()
                {
                    ctx.backend_mut().emit_err(namespace, &err);
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
        namespace: &mut notify::Namespace,
        mut ctx: NeovimCtx<B>,
    ) {
        namespace.push_module(self.module_name);

        let Some(arg) = args.next() else {
            let err = MissingCommandError(self);
            return ctx.backend_mut().emit_err(namespace, &err);
        };

        if let Some(handler) = self.inner.get_mut(arg.as_str()) {
            (handler)(args, namespace, ctx);
        } else if let Some(module) = self.submodules.get_mut(arg.as_str()) {
            module.handle(args, namespace, ctx);
        } else {
            let err = UnknownCommandError(self, arg);
            ctx.backend_mut().emit_err(namespace, &err);
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

        let Some(arg) = args.next() else {
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

impl<K: Ord, V> OrderedMap<K, V> {
    #[inline]
    fn contains_key(&self, key: K) -> bool {
        self.get_idx(&key).is_ok()
    }

    #[inline]
    fn get_idx<Q>(&self, key: &Q) -> Result<usize, usize>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        self.inner
            .binary_search_by(|(probe, _)| Borrow::<Q>::borrow(probe).cmp(key))
    }

    #[inline]
    fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        let idx = self.get_idx(key).ok()?;
        Some(&mut self.inner[idx].1)
    }

    #[inline]
    fn insert(&mut self, key: K, value: V) -> &mut V {
        let idx = self.get_idx(&key).unwrap_or_else(|x| x);
        self.inner.insert(idx, (key, value));
        &mut self.inner[idx].1
    }

    #[inline]
    fn keys(&self) -> impl Iterator<Item = &K> + '_ {
        self.inner.iter().map(|(k, _)| k)
    }
}

impl<K, V> Default for OrderedMap<K, V> {
    #[inline]
    fn default() -> Self {
        Self { inner: Vec::new() }
    }
}

impl<B> notify::Error for MissingCommandError<'_, B> {
    #[inline]
    fn to_level(&self) -> Option<notify::Level> {
        Some(notify::Level::Error)
    }

    #[inline]
    fn to_message(&self) -> notify::Message {
        todo!()
    }
}

impl<B> notify::Error for UnknownCommandError<'_, B> {
    #[inline]
    fn to_level(&self) -> Option<notify::Level> {
        Some(notify::Level::Error)
    }

    #[inline]
    fn to_message(&self) -> notify::Message {
        todo!()
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
        ctx: NeovimCtx<'_, B>,
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
