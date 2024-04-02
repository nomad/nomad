use nomad::prelude::*;

use crate::{Collab, Config, Context, Session, SessionId, SessionState};

/// TODO: docs
#[derive(Clone)]
pub(crate) struct Start {
    config: Get<Config>,
    state: Get<SessionState>,
    set_state: Set<SessionState>,
}

impl Start {
    pub(crate) fn new(ctx: &Context) -> Self {
        Self {
            config: ctx.config.clone(),
            state: ctx.state.clone(),
            set_state: ctx.set_state.clone(),
        }
    }
}

#[async_action]
impl Action<Collab> for Start {
    const NAME: ActionName = action_name!("start");

    type Args = ();

    type Return = ();

    async fn execute(&self, _: ()) -> Result<(), StartError> {
        if let &SessionState::Active(session_id) = self.state.get() {
            return Err(StartError::ExistingSession(session_id));
        }

        let mut session = Session::start(self.config.clone()).await?;

        nvim::print!("Session started with ID {}", session.id());

        self.set_state.set(SessionState::Active(session.id()));

        let _ = session.run().await;

        Ok(())
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
    fn from(err: StartError) -> Self {
        let mut msg = WarningMsg::new();
        msg.add(err.to_string());
        msg
    }
}
