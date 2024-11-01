use nomad::diagnostics::{DiagnosticMessage, HighlightGroup};
use nomad::Shared;

use crate::session::Project;
use crate::session_status::SessionStatus;

/// The type of error returned when a "busy" user tries to start/join a new
/// session.
///
/// The generic parameter represents whether the user was trying to start or
/// join a session when the error occurred.
#[derive(Debug, thiserror::Error)]
pub(crate) enum UserBusyError<const WAS_STARTING: bool> {
    /// Another session is being started.
    #[error(
        "can't start a new session while another is being {}ed",
        if WAS_STARTING { "start" } else { "join" }
    )]
    Starting,

    /// Another session is being joined.
    #[error(
        "can't join a new session while another is being {}ed",
        if WAS_STARTING { "start" } else { "join" }
    )]
    Joining,

    /// The user is already in a session.
    #[error(
        "can't {} a new session, another is already in progress at `{}`",
        if WAS_STARTING { "start" } else { "join" },
        .0.with(|p| p.root().to_string())
    )]
    InSession(Shared<Project>),
}

impl<const WAS_STARTING: bool> TryFrom<&SessionStatus>
    for UserBusyError<WAS_STARTING>
{
    type Error = ();

    fn try_from(status: &SessionStatus) -> Result<Self, Self::Error> {
        match status {
            SessionStatus::Starting => Ok(Self::Starting),
            SessionStatus::Joining(_) => Ok(Self::Joining),
            SessionStatus::InSession(p) => Ok(Self::InSession(p.clone())),
            _ => Err(()),
        }
    }
}

impl<const WAS_STARTING: bool> From<UserBusyError<WAS_STARTING>>
    for DiagnosticMessage
{
    fn from(err: UserBusyError<WAS_STARTING>) -> Self {
        let mut msg = DiagnosticMessage::new();
        let action = if WAS_STARTING { "start" } else { "join" };
        match err {
            UserBusyError::Starting => msg
                .push_str("can't start a new session while another is being ")
                .push_str(action)
                .push_str("ed"),
            UserBusyError::Joining => msg
                .push_str("can't join a new session while another is being ")
                .push_str(action)
                .push_str("ed"),
            UserBusyError::InSession(project) => msg
                .push_str("can't ")
                .push_str(action)
                .push_str(" a new session, another is already in progress at ")
                .push_str_highlighted(
                    project.with(|p| p.root().to_string()),
                    HighlightGroup::special(),
                ),
        };
        msg
    }
}
