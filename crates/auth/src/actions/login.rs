use nvimx::ctx::NeovimCtx;
use nvimx::plugin::{action_name, ActionName, AsyncAction};

use crate::Auth;

#[derive(Clone)]
pub(crate) struct Login {}

impl Login {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl AsyncAction for Login {
    const NAME: ActionName = action_name!("login");
    type Args = ();
    type Docs = ();
    type Module = Auth;

    async fn execute(&mut self, _: Self::Args, _: NeovimCtx<'_>) {}

    fn docs(&self) -> Self::Docs {}
}
