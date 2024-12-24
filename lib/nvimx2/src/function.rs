use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use crate::{Action, ActionName, Backend, MaybeResult, Module, NeovimCtx};

/// TODO: docs.
pub trait Function<B: Backend>: 'static {
    /// TODO: docs.
    const NAME: &'static ActionName;

    /// TODO: docs.
    type Module: Module<B>;

    /// TODO: docs.
    type Args: DeserializeOwned;

    /// TODO: docs.
    type Return: Serialize;

    /// TODO: docs.
    type Docs;

    /// TODO: docs.
    fn call(
        &mut self,
        args: Self::Args,
        ctx: NeovimCtx<'_, B>,
    ) -> impl MaybeResult<Self::Return>;

    /// TODO: docs.
    fn docs() -> Self::Docs;
}

impl<A, B> Function<B> for A
where
    A: Action<B>,
    A::Args: DeserializeOwned,
    A::Return: Serialize,
    B: Backend,
{
    const NAME: &'static ActionName = A::NAME;

    type Module = A::Module;
    type Args = A::Args;
    type Return = A::Return;
    type Docs = A::Docs;

    #[inline]
    fn call(
        &mut self,
        args: A::Args,
        ctx: NeovimCtx<'_, B>,
    ) -> impl MaybeResult<Self::Return> {
        A::call(self, args, ctx)
    }

    #[inline]
    fn docs() -> Self::Docs {
        A::docs()
    }
}
