pub use crate::action_ctx::ActionCtx;
use crate::backend::BackendExt;
use crate::{AsyncCtx, Backend, MaybeResult, Name};

/// TODO: docs.
pub trait Action<B: Backend>: 'static {
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
        ctx: &mut ActionCtx<B>,
    ) -> impl MaybeResult<Self::Return>;
}

/// TODO: docs.
pub trait AsyncAction<B: Backend>: 'static {
    /// TODO: docs.
    const NAME: Name;

    /// TODO: docs.
    type Args;

    /// TODO: docs.
    fn call(
        &mut self,
        args: Self::Args,
        ctx: &mut AsyncCtx<B>,
    ) -> impl Future<Output = impl MaybeResult<()>>;
}

impl<T, B> Action<B> for T
where
    T: AsyncAction<B> + Clone,
    B: Backend,
{
    const NAME: Name = T::NAME;
    type Args = T::Args;
    type Return = ();

    #[inline]
    fn call(&mut self, args: Self::Args, ctx: &mut ActionCtx<B>) {
        let mut this = self.clone();
        let module_path = ctx.module_path().clone();
        ctx.spawn_local(async move |ctx| {
            if let Err(err) = this.call(args, ctx).await.into_result() {
                ctx.with_ctx(move |ctx| {
                    ctx.backend_mut().emit_action_err(
                        &module_path,
                        Self::NAME,
                        err,
                    );
                });
            }
        });
    }
}
