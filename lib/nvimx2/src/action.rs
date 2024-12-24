use crate::{Backend, MaybeResult, Module, NeovimCtx};

/// TODO: docs.
pub trait Action<B: Backend>: 'static {
    /// TODO: docs.
    const NAME: &'static ActionName;

    /// TODO: docs.
    type Module: Module<B>;

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
        ctx: NeovimCtx<'_, B>,
    ) -> impl MaybeResult<Self::Return>;

    /// TODO: docs.
    fn docs() -> Self::Docs;
}

/// TODO: docs.
pub struct ActionName(str);
