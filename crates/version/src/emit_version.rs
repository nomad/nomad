use nvimx2::Plugin;
use nvimx2::action::{Action, ActionCtx};
use nvimx2::backend::Backend;
use nvimx2::command::ToCompletionFn;
use nvimx2::notify::{Message, Name};

use crate::VERSION;

/// TODO: docs.
#[derive(Default)]
pub struct EmitVersion {}

impl EmitVersion {
    /// Creates a new [`EmitVersion`].
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<P, B> Action<P, B> for EmitVersion
where
    P: Plugin<B>,
    B: Backend,
{
    const NAME: Name = "version";

    type Args<'args> = ();
    type Return = ();

    fn call<'s: 's, 'a: 'a>(
        &mut self,
        _: Self::Args<'_>,
        ctx: &mut ActionCtx<P, B>,
    ) {
        ctx.emit_info(Message::from_debug(VERSION));
    }
}

impl<B: Backend> ToCompletionFn<B> for EmitVersion {
    fn to_completion_fn(&self) {}
}
