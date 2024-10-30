use nomad::ctx::NeovimCtx;
use nomad::{action_name, Action, ActionName, Shared};

use crate::session_status::SessionStatus;
use crate::Collab;

#[derive(Clone)]
pub(crate) struct Start {
    session_status: Shared<SessionStatus>,
}

impl Start {
    pub(crate) fn new(session_status: Shared<SessionStatus>) -> Self {
        Self { session_status }
    }
}

impl<'a> Action<NeovimCtx<'a>> for Start {
    const NAME: ActionName = action_name!("start");
    type Args = ();
    type Docs = ();
    type Module = Collab;
    type Return = ();

    fn execute(&mut self, _args: Self::Args, _: NeovimCtx<'a>) {
        todo!()
    }

    fn docs(&self) -> Self::Docs {}
}
