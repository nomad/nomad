use nvimx2::command::Command;
use nvimx2::notify::Message;
use nvimx2::{ActionCtx, Backend, Name};

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

impl<B: Backend> Command<B> for EmitVersion {
    const NAME: Name = "version";

    type Args = ();

    fn call(&mut self, _: Self::Args, ctx: &mut ActionCtx<B>) {
        ctx.emit_info(Message::from_str(format!("{VERSION:?}")));
    }
}
