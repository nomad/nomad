use ed::action::Action;
use ed::backend::Backend;
use ed::command::ToCompletionFn;
use ed::notify::{Message, Name};
use ed::{Borrowed, Context};

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
        ctx: &mut Context<B, Borrowed>,
    ) {
        ctx.emit_info(Message::from_display(VERSION));
    }
}

impl<B: Backend> ToCompletionFn<B> for EmitVersion {
    fn to_completion_fn(&self) {}
}
