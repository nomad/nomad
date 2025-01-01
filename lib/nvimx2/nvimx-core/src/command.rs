//! TODO: docs.

use smol_str::SmolStr;

use crate::{
    Action,
    ActionName,
    Backend,
    ByteOffset,
    MaybeResult,
    NeovimCtx,
    notify,
};

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
pub struct CommandCompletion {
    inner: SmolStr,
}

impl<'a> CommandArgs<'a> {
    /// TODO: docs.
    #[inline]
    pub fn new(_command_str: &'a str) -> Self {
        todo!()
    }

    /// TODO: docs.
    #[inline]
    pub fn next(&mut self) -> Option<&'a str> {
        todo!()
    }
}

impl CommandCompletion {
    /// TODO: docs.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
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
