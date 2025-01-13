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
    // - https://rustc-dev-guide.rust-lang.org/early_late_parameters.html#must-be-constrained-by-argument-types
    fn call<'slf: 'slf, 'args: 'args>(
        &'slf mut self,
        args: Self::Args<'args>,
        ctx: &mut ActionCtx<P, B>,
    ) -> impl MaybeResult<Self::Return, B> + use<'slf, 'args, Self, P, B>;
}
