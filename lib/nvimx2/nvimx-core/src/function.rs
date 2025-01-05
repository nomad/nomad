use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use crate::{Action, ActionCtx, ActionName, Backend, MaybeResult};

/// TODO: docs.
pub trait Function<B: Backend>: 'static {
    /// TODO: docs.
    const NAME: &'static ActionName;

    /// TODO: docs.
    type Args: DeserializeOwned;

    /// TODO: docs.
    type Return: Serialize + 'static;

    /// TODO: docs.
    type Docs;

    /// TODO: docs.
    fn call(
        &mut self,
        args: Self::Args,
        ctx: &mut ActionCtx<B>,
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

    type Args = A::Args;
    type Return = A::Return;
    type Docs = A::Docs;

    #[inline]
    fn call(
        &mut self,
        args: A::Args,
        ctx: &mut ActionCtx<B>,
    ) -> impl MaybeResult<Self::Return> {
        A::call(self, args, ctx)
    }

    #[inline]
    fn docs() -> Self::Docs {
        A::docs()
    }
}
