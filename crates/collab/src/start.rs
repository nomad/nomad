use nomad::prelude::*;

use crate::{Activity, Collab, Config, Session, SessionId};

/// TODO: docs
#[derive(Clone)]
pub(crate) struct Start {
    activity: Shared<Activity>,
    config: Get<Config>,
}

impl Start {
    pub(crate) fn new(collab: &Collab) -> Self {
        Self {
            activity: collab.activity.clone(),
            config: collab.config.clone(),
        }
    }
}

#[async_action]
impl Action<Collab> for Start {
    const NAME: ActionName = action_name!("start");

    type Args = ();

    type Return = ();

    async fn execute(&mut self, _: ()) -> Result<(), StartError> {
        match self.activity.get() {
            Activity::Active(id) => return Err(StartError::AlreadyActive(id)),
            Activity::Joining => return Err(StartError::AlreadyJoining),
            _ => (),
        }

        self.activity.set(Activity::Starting);

        // TODO: there should be a reactor that looks at the activity and
        // prints a message when it changes.
        // Self::info(SessionStarting);

        let mut session = Session::start(&self.config).await?;

        let session_id = session.id();

        self.activity.set(Activity::Active(session_id));

        clipboard::set(session_id)?;

        // TODO: there should be a reactor that looks at the activity and
        // prints a message when it changes.
        // Self::info(SessionStarted);

        nvim::print!("Session ID copied to clipboard");

        let _ = session.run().await;

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error("there is already an active session with ID {0}")]
    AlreadyActive(SessionId),

    #[error("cannot start a session while another one is being joined")]
    AlreadyJoining,

    #[error(transparent)]
    Clipboard(#[from] clipboard::ClipboardError),

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
