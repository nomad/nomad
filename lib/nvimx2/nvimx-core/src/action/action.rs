use crate::action::ActionCtx;
use crate::backend::Backend;
use crate::notify::{MaybeResult, Name};
use crate::plugin::Plugin;

/// TODO: docs.
pub trait Action<P, B>: 'static
where
    P: Plugin<B>,
    B: Backend,
{
    /// TODO: docs.
    const NAME: Name;

    /// TODO: docs.
    type Args;

    /// TODO: docs.
    type Return;

    /// TODO: docs.
    fn call(
        &mut self,
        args: Self::Args,
        ctx: &mut ActionCtx<P, B>,
    ) -> impl MaybeResult<Self::Return, B>;
}
