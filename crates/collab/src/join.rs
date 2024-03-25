use nomad::prelude::*;

use crate::{Collab, Config, SessionId};

pub(crate) struct Join {
    _config: Get<Config>,
}

impl Join {
    pub(crate) fn new(config: Get<Config>) -> Self {
        Self { _config: config }
    }
}

#[async_action]
impl Action<Collab> for Join {
    const NAME: ActionName = action_name!("join");

    type Args = SessionId;

    type Return = ();

    async fn execute(&self, _session_id: SessionId) {}
}
