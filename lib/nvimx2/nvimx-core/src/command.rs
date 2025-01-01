//! TODO: docs.

use smol_str::SmolStr;

use crate::{Action, ActionName, Backend, MaybeResult, NeovimCtx, notify};

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
    fn docs() -> Self::Docs;
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
    A: Action<B, Return = ()>,
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
    fn docs() -> Self::Docs {
        A::docs()
    }
}
