use nvimx::ctx::NeovimCtx;
use nvimx::plugin::{action_name, ActionName, AsyncAction};

use crate::Auth;

#[derive(Clone)]
pub(crate) struct Logout {}

impl Logout {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl AsyncAction for Logout {
    const NAME: ActionName = action_name!("logout");
    type Args = ();
    type Docs = ();
    type Module = Auth;

    async fn execute(&mut self, _: Self::Args, _: NeovimCtx<'_>) {}

    fn docs(&self) -> Self::Docs {}
}
