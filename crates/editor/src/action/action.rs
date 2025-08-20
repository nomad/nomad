use crate::{Borrowed, Context, Editor};

/// TODO: docs.
pub trait Action<Ed: Editor>: 'static {
    /// TODO: docs.
    const NAME: &str;

    /// TODO: docs.
    type Args<'args>;

    /// TODO: docs.
    type Return;

    /// TODO: docs.
    fn call(
        &mut self,
        args: Self::Args<'_>,
        ctx: &mut Context<Ed, Borrowed<'_>>,
    ) -> Self::Return;
}
