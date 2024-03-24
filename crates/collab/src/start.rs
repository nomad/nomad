use collab::messages::SessionId;
use nomad::prelude::*;

use crate::{Collab, Config, Session, SessionState};

/// TODO: docs
#[derive(Clone)]
pub(crate) struct Start {
    config: Get<Config>,

    /// The current collab session, if there is one.
    state: Get<SessionState>,

    /// TODO: docs
    set_state: Set<SessionState>,
}

impl Start {
    async fn async_execute(&self) -> Result<(), StartError> {
        if let SessionState::Active(session) = self.state.get() {
            return Err(StartError::ExistingSession(session.id()));
        }

        let session = Session::start(self.config.clone()).await?;

        self.set_state.set(SessionState::Active(session));

        Ok(())
    }
}

impl Action<Collab> for Start {
    const NAME: ActionName = action_name!("join");

    type Args = ();

    type Return = ();

    fn execute(&self, _: ()) {
        let this = self.clone();

        spawn(async move {
            let _ = this.async_execute().await;
        })
        .detach();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error("there is already an active session with ID {0}")]
    ExistingSession(SessionId),

    #[error(transparent)]
    Start(#[from] crate::session::StartError),
}

impl From<StartError> for WarningMsg {
    fn from(_err: StartError) -> Self {
        todo!();
    }
}
