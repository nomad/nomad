use nomad::ctx::NeovimCtx;
use nomad::{action_name, Action, ActionName, Shared};

use crate::session_status::SessionStatus;
use crate::Collab;

#[derive(Clone)]
pub(crate) struct Join {
    session_status: Shared<SessionStatus>,
}

impl Join {
    pub(crate) fn new(session_status: Shared<SessionStatus>) -> Self {
        Self { session_status }
    }
}

impl<'a> Action<NeovimCtx<'a>> for Join {
    const NAME: ActionName = action_name!("join");
    type Args = ();
    type Docs = ();
    type Module = Collab;
    type Return = ();

    fn execute(&mut self, _args: Self::Args, _: NeovimCtx<'a>) {
        todo!()
    }

    fn docs(&self) -> Self::Docs {}
}
