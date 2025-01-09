use crate::AsyncCtx;
use crate::action::{Action, ActionCtx};
use crate::backend::{Backend, BackendExt};
use crate::notify::{self, MaybeResult, Name};
use crate::plugin::Plugin;

/// TODO: docs.
pub trait AsyncAction<P, B>: 'static
where
    P: Plugin<B>,
    B: Backend,
{
    /// TODO: docs.
    const NAME: Name;

    /// TODO: docs.
    type Args;

    /// TODO: docs.
    fn call(
        &mut self,
        args: Self::Args,
        ctx: &mut AsyncCtx<P, B>,
    ) -> impl Future<Output = impl MaybeResult<(), B>>;
}

impl<T, P, B> Action<P, B> for T
where
    T: AsyncAction<P, B> + Clone,
    P: Plugin<B>,
    B: Backend,
{
    const NAME: Name = T::NAME;
    type Args = T::Args;
    type Return = ();

    #[inline]
    fn call(&mut self, args: Self::Args, ctx: &mut ActionCtx<P, B>) {
        let mut this = self.clone();
        let module_path = ctx.module_path().clone();
        ctx.spawn_local(async move |ctx| {
            if let Err(err) = this.call(args, ctx).await.into_result() {
                ctx.with_ctx(move |ctx| {
                    ctx.backend_mut().emit_err::<P, _>(
                        notify::Source {
                            module_path: &module_path,
                            action_name: Some(Self::NAME),
                        },
                        err,
                    );
                });
            }
        });
    }
}
