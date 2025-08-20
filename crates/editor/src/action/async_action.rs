use crate::action::Action;
use crate::{Borrowed, Context, Editor};

/// TODO: docs.
pub trait AsyncAction<Ed: Editor>: 'static {
    /// TODO: docs.
    const NAME: &str;

    /// TODO: docs.
    type Args;

    /// TODO: docs.
    fn call(
        &mut self,
        args: Self::Args,
        ctx: &mut Context<Ed>,
    ) -> impl Future<Output = ()>;
}

impl<T, Ed> Action<Ed> for T
where
    T: AsyncAction<Ed> + Clone,
    Ed: Editor,
{
    const NAME: &str = T::NAME;

    type Args<'args> = T::Args;
    type Return = ();

    #[inline]
    fn call(
        &mut self,
        args: Self::Args<'_>,
        ctx: &mut Context<Ed, Borrowed<'_>>,
    ) {
        let mut this = self.clone();
        ctx.spawn_and_detach(async move |ctx| {
            this.call(args, ctx).await;
        });
    }
}
