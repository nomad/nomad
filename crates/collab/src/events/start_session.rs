use nomad::neovim::{self, Neovim};

use crate::NeovimCollab;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StartSession;

impl StartSession {
    pub(crate) const NAME: &str = "start";
}

impl neovim::Command for StartSession {
    const NAME: &str = Self::NAME;
    type Args = ();
    type Module = NeovimCollab;
}

impl neovim::Function for StartSession {
    const NAME: &str = Self::NAME;
    type Args = ();
    type Module = NeovimCollab;
}
