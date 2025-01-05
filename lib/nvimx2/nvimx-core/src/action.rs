pub use crate::action_ctx::ActionCtx;
use crate::backend::BackendExt;
use crate::{AsyncCtx, Backend, MaybeResult};

/// TODO: docs.
pub trait Action<B: Backend>: 'static {
    /// TODO: docs.
    const NAME: ActionName;

    /// TODO: docs.
    type Args;

    /// TODO: docs.
    type Return;

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

/// TODO: docs.
pub trait AsyncAction<B: Backend>: 'static {
    /// TODO: docs.
    const NAME: ActionName;

    /// TODO: docs.
    type Args;

    /// TODO: docs.
    type Docs;

    /// TODO: docs.
    fn call(
        &mut self,
        args: Self::Args,
        ctx: &mut AsyncCtx<B>,
    ) -> impl Future<Output = impl MaybeResult<()>>;

    /// TODO: docs.
    fn docs() -> Self::Docs;
}

/// TODO: docs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ActionName(&'static str);

impl ActionName {
    /// TODO: docs.
    #[inline]
    pub const fn as_str(self) -> &'static str {
        self.0
    }

    /// TODO: docs.
    #[inline]
    pub const fn new(name: &'static str) -> Self {
        assert!(!name.is_empty());
        Self(name)
    }
}

impl<T, B> Action<B> for T
where
    T: AsyncAction<B> + Clone,
    B: Backend,
{
    const NAME: ActionName = T::NAME;
    type Args = T::Args;
    type Return = ();
    type Docs = T::Docs;

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

    #[inline]
    fn docs() -> Self::Docs {
        T::docs()
    }
}
