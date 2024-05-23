use nomad::prelude::*;

use crate::{Activity, Collab, Config, Session, SessionId};

#[derive(Clone)]
pub(crate) struct Join {
    activity: Shared<Activity>,
    config: Get<Config>,
}

impl Join {
    pub(crate) fn new(collab: &Collab) -> Self {
        Self {
            activity: collab.activity.clone(),
            config: collab.config.clone(),
        }
    }
}

#[async_action]
impl Action<Collab> for Join {
    const NAME: ActionName = action_name!("join");

    type Args = SessionId;

    type Return = ();

    async fn execute(&mut self, id: SessionId) -> Result<(), JoinError> {
        match self.activity.get() {
            Activity::Active(id) => return Err(JoinError::AlreadyActive(id)),
            Activity::Starting => return Err(JoinError::AlreadyStarting),
            _ => (),
        }

        self.activity.set(Activity::Joining);

        let mut session = Session::join(&self.config, id).await?;

        self.activity.set(Activity::Active(session.id()));

        let _ = session.run().await;

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JoinError {
    #[error("there is already an active session with ID {0}")]
    AlreadyActive(SessionId),

    #[error("cannot join a session while another one is being started")]
    AlreadyStarting,

    #[error(transparent)]
    Join(#[from] crate::session::JoinError),
}

impl From<JoinError> for WarningMsg {
    fn from(err: JoinError) -> Self {
        let mut msg = WarningMsg::new();
        msg.add(err.to_string());
        msg
    }
}
