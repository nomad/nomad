use nvimx2::NeovimCtx;
use nvimx2::action::Action;
use nvimx2::backend::Backend;
use nvimx2::command::ToCompletionFn;
use nvimx2::notify::{Message, Name};

use crate::VERSION;

/// TODO: docs.
#[derive(Default)]
pub struct EmitVersion {}

impl EmitVersion {
    /// Creates a new [`EmitVersion`].
    pub fn new() -> Self {
        Self::default()
    }
}

impl<B: Backend> Action<B> for EmitVersion {
    const NAME: Name = "version";

    type Args<'args> = ();
    type Return = ();

    fn call<'s: 's, 'a: 'a>(
        &mut self,
        _: Self::Args<'_>,
        ctx: &mut NeovimCtx<B>,
    ) {
        ctx.emit_info(Message::from_display(VERSION));
    }
}

impl<B: Backend> ToCompletionFn<B> for EmitVersion {
    fn to_completion_fn(&self) {}
}
