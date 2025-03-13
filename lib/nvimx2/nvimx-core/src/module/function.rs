use serde::de::Deserialize;
use serde::ser::Serialize;

use crate::EditorCtx;
use crate::action::Action;
use crate::backend::Backend;
use crate::notify::{MaybeResult, Name};

/// TODO: docs.
pub trait Function<B: Backend>: 'static {
    /// TODO: docs.
    const NAME: Name;

    /// TODO: docs.
    type Args<'args>: Deserialize<'args>;

    /// TODO: docs.
    type Return: Serialize;

    /// TODO: docs.
    fn call<'this, 'args>(
        &'this mut self,
        args: Self::Args<'args>,
        ctx: &mut EditorCtx<B>,
    ) -> impl MaybeResult<Self::Return> + use<'this, 'args, Self, B>;
}

impl<A, B> Function<B> for A
where
    A: Action<B>,
    for<'args> A::Args<'args>: Deserialize<'args>,
    A::Return: Serialize,
    B: Backend,
{
    const NAME: Name = A::NAME;

    type Args<'a> = A::Args<'a>;
    type Return = A::Return;

    #[inline]
    fn call<'this, 'args>(
        &'this mut self,
        args: A::Args<'args>,
        ctx: &mut EditorCtx<B>,
    ) -> impl MaybeResult<Self::Return> + use<'this, 'args, A, B> {
        A::call(self, args, ctx)
    }
}
