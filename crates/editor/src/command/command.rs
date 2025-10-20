use smol_str::SmolStr;

use crate::command::CommandArgs;
use crate::context::{Borrowed, Context};
use crate::editor::{ByteOffset, Editor};
use crate::module::Action;
use crate::notify;

/// TODO: docs.
pub trait Command<Ed: Editor>: 'static {
    /// TODO: docs.
    const NAME: &str;

    /// TODO: docs.
    type Args<'args>: TryFrom<CommandArgs<'args>, Error: notify::Error>;

    /// TODO: docs.
    fn call(
        &mut self,
        args: Self::Args<'_>,
        ctx: &mut Context<Ed, Borrowed<'_>>,
    );

    /// TODO: docs.
    fn to_completion_fn(&self) -> impl CompletionFn + 'static {}
}

/// TODO: docs.
pub trait CompletionFn {
    /// TODO: docs.
    fn call(
        &mut self,
        args: CommandArgs<ByteOffset>,
    ) -> impl IntoIterator<Item = CommandCompletion>;
}

/// TODO: docs.
pub trait ToCompletionFn<Ed: Editor> {
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

impl<A, Ed> Command<Ed> for A
where
    A: Action<Ed, Return = ()> + ToCompletionFn<Ed>,
    for<'a> A::Args<'a>: TryFrom<CommandArgs<'a>, Error: notify::Error>,
    Ed: Editor,
{
    const NAME: &str = A::NAME;

    type Args<'a> = A::Args<'a>;

    #[inline]
    fn call(
        &mut self,
        args: Self::Args<'_>,
        ctx: &mut Context<Ed, Borrowed<'_>>,
    ) {
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
        _: CommandArgs<ByteOffset>,
    ) -> impl IntoIterator<Item = CommandCompletion> {
        core::iter::empty::<CommandCompletion>()
    }
}

impl<F, R> CompletionFn for F
where
    F: FnMut(CommandArgs<ByteOffset>) -> R,
    R: IntoIterator<Item = CommandCompletion>,
{
    #[inline]
    fn call(
        &mut self,
        args: CommandArgs<ByteOffset>,
    ) -> impl IntoIterator<Item = CommandCompletion> {
        (self)(args)
    }
}
