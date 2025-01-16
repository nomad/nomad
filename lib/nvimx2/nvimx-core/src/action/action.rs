use crate::NeovimCtx;
use crate::backend::Backend;
use crate::notify::{MaybeResult, Name};

/// TODO: docs.
pub trait Action<B: Backend>: 'static {
    /// TODO: docs.
    const NAME: Name;

    /// TODO: docs.
    type Args<'args>;

    /// TODO: docs.
    type Return;

    /// TODO: docs.
    //
    // The useless `'x: 'x` bounds make the parameters early-bound, which
    // allows implementors to refine the method's output (to e.g. return ())
    // instead of repeating the whole `impl .. + use<..>` shebang every time.
    //
    // See:
    //
    // - https://github.com/rust-lang/rust/issues/87803
    // - https://github.com/rust-lang/rust/issues/109476
    // - https://rustc-dev-guide.rust-lang.org/early_late_parameters.html
    fn call<'slf: 'slf, 'args: 'args>(
        &'slf mut self,
        args: Self::Args<'args>,
        ctx: &mut NeovimCtx<B>,
    ) -> impl MaybeResult<Self::Return> + use<'slf, 'args, Self, B>;
}
