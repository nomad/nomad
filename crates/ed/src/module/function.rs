use serde::de::Deserialize;
use serde::ser::Serialize;

use crate::action::Action;
use crate::notify::{MaybeResult, Name};
use crate::{Borrowed, Context, Editor};

/// TODO: docs.
pub trait Function<Ed: Editor>: 'static {
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
        ctx: &mut Context<Ed, Borrowed<'_>>,
    ) -> impl MaybeResult<Self::Return> + use<'this, 'args, Self, Ed>;
}

impl<A, Ed> Function<Ed> for A
where
    A: Action<Ed>,
    for<'args> A::Args<'args>: Deserialize<'args>,
    A::Return: Serialize,
    Ed: Editor,
{
    const NAME: Name = A::NAME;

    type Args<'a> = A::Args<'a>;
    type Return = A::Return;

    #[inline]
    fn call<'this, 'args>(
        &'this mut self,
        args: A::Args<'args>,
        ctx: &mut Context<Ed, Borrowed<'_>>,
    ) -> impl MaybeResult<Self::Return> + use<'this, 'args, A, Ed> {
        A::call(self, args, ctx)
    }
}
