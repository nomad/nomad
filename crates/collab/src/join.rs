use nomad::prelude::*;

use crate::{Collab, Config, SessionId};

pub(crate) struct Join {
    _config: Get<Config>,
}

impl Join {
    pub(crate) fn new(config: Get<Config>) -> Self {
        Self { _config: config }
    }

    async fn async_execute(&self, session_id: SessionId) {}
}

// Q: how do we forbid `Return` from being anything other than `()` if `execute`
// is async?

impl Action<Collab> for Join {
    const NAME: ActionName = action_name!("join");

    type Args = SessionId;

    type Return = ();

    fn execute(
        &self,
        _session_id: SessionId,
    ) -> impl MaybeFuture<Output = ()> {
        MaybeFutureEnum::from(self.async_execute(_session_id))
    }
}
