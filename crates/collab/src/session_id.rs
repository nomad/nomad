use std::fmt::{self, Display};
use std::str::FromStr;

use collab_client::messages::SessionId as CollabSessionId;
use nomad::prelude::{CommandArgs, WarningMsg};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub struct SessionId(CollabSessionId);

impl From<CollabSessionId> for SessionId {
    fn from(id: CollabSessionId) -> Self {
        Self(id)
    }
}

impl From<SessionId> for CollabSessionId {
    fn from(id: SessionId) -> Self {
        id.0
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for SessionId {
    type Err = <CollabSessionId as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CollabSessionId::from_str(s).map(Self)
    }
}

impl TryFrom<CommandArgs> for SessionId {
    type Error = SessionIdFromArgsError;

    fn try_from(args: CommandArgs) -> Result<Self, Self::Error> {
        match args.as_slice() {
            [arg] => arg.parse().map_err(Into::into),
            [] => Err(Self::Error::NoArgs),
            args => Err(Self::Error::TooManyArgs { num_args: args.len() }),
        }
    }
}

/// Errors that can occur when converting [`CommandArgs`] into a[`SessionId`].
#[derive(Debug, thiserror::Error)]
pub enum SessionIdFromArgsError {
    /// The command arguments were empty.
    #[error("expected a session ID")]
    NoArgs,

    /// The command arguments contained an invalid session ID.
    #[error(transparent)]
    InvalidArg(#[from] collab_client::SessionIdFromStrError),

    /// The command arguments contained more than one argument.
    #[error("expected a session ID")]
    TooManyArgs { num_args: usize },
}

impl From<SessionIdFromArgsError> for WarningMsg {
    fn from(err: SessionIdFromArgsError) -> Self {
        let mut msg = WarningMsg::new();
        msg.add(err.to_string());
        msg
    }
}
