use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use crate::{Action, ActionCtx, Backend, MaybeResult, Name};

/// TODO: docs.
pub trait Function<B: Backend>: 'static {
    /// TODO: docs.
    const NAME: Name;

    /// TODO: docs.
    type Args: DeserializeOwned;

    /// TODO: docs.
    type Return: Serialize + 'static;

    /// TODO: docs.
    fn call(
        &mut self,
        args: Self::Args,
        ctx: &mut ActionCtx<B>,
    ) -> impl MaybeResult<Self::Return>;
}

impl<A, B> Function<B> for A
where
    A: Action<B>,
    A::Args: DeserializeOwned,
    A::Return: Serialize,
    B: Backend,
{
    const NAME: Name = A::NAME;

    type Args = A::Args;
    type Return = A::Return;

    #[inline]
    fn call(
        &mut self,
        args: A::Args,
        ctx: &mut ActionCtx<B>,
    ) -> impl MaybeResult<Self::Return> {
        A::call(self, args, ctx)
    }
}
