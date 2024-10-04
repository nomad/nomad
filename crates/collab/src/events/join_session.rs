use nomad::neovim::{self, Neovim};

use crate::{NeovimCollab, SessionId};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct JoinSession;

impl JoinSession {
    pub(crate) const NAME: &str = "join";
}

impl neovim::Command for JoinSession {
    const NAME: &str = Self::NAME;
    type Args = SessionId;
    type Module = NeovimCollab;
}

impl neovim::Function for JoinSession {
    const NAME: &str = Self::NAME;
    type Args = SessionId;
    type Module = NeovimCollab;
}
