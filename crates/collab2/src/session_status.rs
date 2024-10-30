use collab_server::SessionId;
use nomad::Shared;

use crate::session::Project;

#[derive(Default)]
pub(crate) enum SessionStatus {
    /// The user is not in a session.
    #[default]
    NotInSession,

    /// The user is starting a new session, but it hasn't been fully
    /// setup yet.
    Starting,

    /// The user is joining an existing session, but it hasn't been fully
    /// setup yet.
    Joining(SessionId),

    /// The user is participating in a session with the given [`Project`].
    InSession(Shared<Project>),
}
