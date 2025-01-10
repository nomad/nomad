use smol_str::SmolStr;

use crate::ByteOffset;
use crate::action::{Action, ActionCtx};
use crate::backend::Backend;
use crate::command::CommandArgs;
use crate::notify::{self, MaybeResult, Name};
use crate::plugin::Plugin;

/// TODO: docs.
pub trait Command<P, B>: 'static
where
    P: Plugin<B>,
    B: Backend,
{
    /// TODO: docs.
    const NAME: Name;

    /// TODO: docs.
    type Args: for<'args> TryFrom<CommandArgs<'args>, Error: notify::Error<B>>;

    /// TODO: docs.
    fn call(
        &mut self,
        args: Self::Args,
        ctx: &mut ActionCtx<P, B>,
    ) -> impl MaybeResult<(), B>;

    /// TODO: docs.
    fn to_completion_fn(&self) -> impl CompletionFn + 'static {}
}

/// TODO: docs.
pub trait CompletionFn {
    /// TODO: docs.
    fn call(
        &mut self,
        args: CommandArgs,
        offset: ByteOffset,
    ) -> impl IntoIterator<Item = CommandCompletion>;
}

/// TODO: docs.
pub trait ToCompletionFn<B: Backend> {
    /// TODO: docs.
    fn to_completion_fn(&self) -> impl CompletionFn + 'static;
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

impl<A, P, B> Command<P, B> for A
where
    A: Action<P, B, Return = ()> + ToCompletionFn<B>,
    A::Args: for<'args> TryFrom<CommandArgs<'args>, Error: notify::Error<B>>,
    P: Plugin<B>,
    B: Backend,
{
    const NAME: Name = A::NAME;

    type Args = A::Args;

    #[inline]
    fn call(
        &mut self,
        args: Self::Args,
        ctx: &mut ActionCtx<P, B>,
    ) -> impl MaybeResult<(), B> {
        A::call(self, args, ctx)
    }

    #[inline]
    fn to_completion_fn(&self) -> impl CompletionFn + 'static {
        ToCompletionFn::to_completion_fn(self)
    }
}

impl CompletionFn for () {
    #[inline]
    fn call(
        &mut self,
        _: CommandArgs,
        _: ByteOffset,
    ) -> impl IntoIterator<Item = CommandCompletion> {
        core::iter::empty::<CommandCompletion>()
    }
}

impl<F, R> CompletionFn for F
where
    F: FnMut(CommandArgs, ByteOffset) -> R,
    R: IntoIterator<Item = CommandCompletion>,
{
    #[inline]
    fn call(
        &mut self,
        args: CommandArgs,
        offset: ByteOffset,
    ) -> impl IntoIterator<Item = CommandCompletion> {
        (self)(args, offset)
    }
}
