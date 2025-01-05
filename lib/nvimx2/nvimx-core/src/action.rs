pub use crate::action_ctx::ActionCtx;
use crate::backend::BackendExt;
use crate::{AsyncCtx, Backend, MaybeResult};

/// TODO: docs.
pub trait Action<B: Backend>: 'static {
    /// TODO: docs.
    const NAME: &'static ActionName;

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
    const NAME: &'static ActionName;

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
#[derive(Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ActionName(str);

impl ActionName {
    /// TODO: docs.
    #[inline]
    pub const fn as_str(&self) -> &str {
        &self.0
    }

    /// TODO: docs.
    #[inline]
    pub const fn new(name: &str) -> &Self {
        assert!(!name.is_empty());
        assert!(name.len() <= 24);
        // SAFETY: `ActionName` is a `repr(transparent)` newtype around `str`.
        unsafe { &*(name as *const str as *const Self) }
    }
}

impl<T, B> Action<B> for T
where
    T: AsyncAction<B> + Clone,
    B: Backend,
{
    const NAME: &'static ActionName = T::NAME;
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
                })
                .await;
            }
        })
        .detach();
    }

    #[inline]
    fn docs() -> Self::Docs {
        T::docs()
    }
}
