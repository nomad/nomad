use nomad::prelude::*;

use crate::{Collab, Config, Session, SessionId, SessionState};

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
        if let &SessionState::Active(session_id) = self.state.get() {
            return Err(StartError::ExistingSession(session_id));
        }

        let session = Session::start(self.config.clone()).await?;

        self.set_state.set(SessionState::Active(session.id()));

        Ok(())
    }

    pub(crate) fn new(config: Get<Config>) -> Self {
        let (state, set_state) = new_input(SessionState::Inactive);
        Self { config, state, set_state }
    }
}

impl Action<Collab> for Start {
    const NAME: ActionName = action_name!("join");

    type Args = ();

    type Return = ();

    fn execute(
        &self,
        _: (),
    ) -> impl MaybeFuture<Output = Result<(), StartError>> {
        MaybeFutureEnum::from(self.async_execute())
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
