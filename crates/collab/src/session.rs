use collab::messages::SessionId;
use nomad::prelude::Get;

use crate::config::ConnectorError;
use crate::Config;

/// TODO: docs
pub(crate) struct Session {
    /// TODO: docs
    id: SessionId,

    /// TODO: docs
    receiver: collab::Receiver,

    /// TODO: docs
    sender: collab::Sender,
}

impl Session {
    /// Returns the [`SessionId`] of the session, which is unique to each
    /// session and can be sent to other peers to join the session.
    pub(crate) fn id(&self) -> SessionId {
        self.id
    }

    pub async fn start(config: Get<Config>) -> Result<Self, StartError> {
        let (sender, receiver, session_id) =
            config.get().connector()?.start().await?;

        Ok(Self { id: session_id, receiver, sender })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error(transparent)]
    Connection(#[from] collab::Error),

    #[error(transparent)]
    Connector(#[from] ConnectorError),
}

/// Whether there is an active collab session or not.
pub(crate) enum SessionState {
    /// There is an active collab session.
    Active(Session),

    /// There is no active collab session.
    Inactive,
}
