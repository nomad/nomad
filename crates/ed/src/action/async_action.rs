use crate::action::Action;
use crate::backend::Backend;
use crate::notify::{MaybeResult, Name};
use crate::{Borrowed, Context};

/// TODO: docs.
pub trait AsyncAction<B: Backend>: 'static {
    /// TODO: docs.
    const NAME: Name;

    /// TODO: docs.
    type Args;

    /// TODO: docs.
    fn call<'this>(
        &'this mut self,
        args: Self::Args,
        ctx: &mut Context<B>,
    ) -> impl Future<Output = impl MaybeResult<()> + 'this>;
}

impl<T, B> Action<B> for T
where
    T: AsyncAction<B> + Clone,
    B: Backend,
{
    const NAME: Name = T::NAME;

    type Args<'args> = T::Args;
    type Return = ();

    #[inline]
    fn call<'s: 's, 'a: 'a>(
        &mut self,
        args: Self::Args<'_>,
        ctx: &mut Context<B, Borrowed<'_>>,
    ) {
        let mut this = self.clone();
        ctx.spawn_and_detach(async move |ctx| {
            if let Err(err) = this.call(args, ctx).await.into_result() {
                ctx.emit_err(err);
            }
        });
    }
}
