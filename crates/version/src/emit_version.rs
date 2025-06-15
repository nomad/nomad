use ed::action::Action;
use ed::command::ToCompletionFn;
use ed::notify::{Message, Name};
use ed::{Borrowed, Context, Editor};

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

impl<Ed: Editor> Action<Ed> for EmitVersion {
    const NAME: Name = "version";

    type Args<'args> = ();
    type Return = ();

    fn call<'s: 's, 'a: 'a>(
        &mut self,
        _: Self::Args<'_>,
        ctx: &mut Context<Ed, Borrowed>,
    ) {
        ctx.emit_info(Message::from_display(VERSION));
    }
}

impl<Ed: Editor> ToCompletionFn<Ed> for EmitVersion {
    fn to_completion_fn(&self) {}
}
