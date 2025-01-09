use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use crate::action::{Action, ActionCtx};
use crate::backend::Backend;
use crate::notify::{MaybeResult, Name};
use crate::plugin::Plugin;

/// TODO: docs.
pub trait Function<P: Plugin<B>, B: Backend>: 'static {
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
        ctx: &mut ActionCtx<P, B>,
    ) -> impl MaybeResult<Self::Return, B>;
}

impl<A, P, B> Function<P, B> for A
where
    A: Action<P, B>,
    A::Args: DeserializeOwned,
    A::Return: Serialize,
    P: Plugin<B>,
    B: Backend,
{
    const NAME: Name = A::NAME;

    type Args = A::Args;
    type Return = A::Return;

    #[inline]
    fn call(
        &mut self,
        args: A::Args,
        ctx: &mut ActionCtx<P, B>,
    ) -> impl MaybeResult<Self::Return, B> {
        A::call(self, args, ctx)
    }
}
