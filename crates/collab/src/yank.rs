//! TODO: docs.

use ed::Context;
use ed::action::AsyncAction;
use ed::command::ToCompletionFn;
use ed::notify::{self, Name};

use crate::collab::Collab;
use crate::editors::{ActionForSelectedSession, CollabEditor};
use crate::project::{NoActiveSessionError, Projects};

/// An `Action` that pastes the [`SessionId`] of any active session to the
/// user's clipboard.
#[derive(cauchy::Clone)]
pub struct Yank<Ed: CollabEditor> {
    projects: Projects<Ed>,
}

impl<Ed: CollabEditor> AsyncAction<Ed> for Yank<Ed> {
    const NAME: Name = "yank";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut Context<Ed>,
    ) -> Result<(), YankError<Ed>> {
        let Some((_, session_id)) = self
            .projects
            .select(ActionForSelectedSession::CopySessionId, ctx)
            .await
            .map_err(YankError::NoActiveSession)?
        else {
            return Ok(());
        };

        Ed::copy_session_id(session_id, ctx)
            .await
            .map_err(YankError::PasteSessionId)
    }
}

/// The type of error that can occur when [`Yank`]ing fails.
pub enum YankError<Ed: CollabEditor> {
    /// TODO: docs.
    NoActiveSession(NoActiveSessionError<Ed>),

    /// TODO: docs.
    PasteSessionId(Ed::CopySessionIdError),
}

impl<Ed: CollabEditor> From<&Collab<Ed>> for Yank<Ed> {
    fn from(collab: &Collab<Ed>) -> Self {
        Self { projects: collab.projects.clone() }
    }
}

impl<Ed: CollabEditor> ToCompletionFn<Ed> for Yank<Ed> {
    fn to_completion_fn(&self) {}
}

impl<Ed: CollabEditor> notify::Error for YankError<Ed> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            YankError::NoActiveSession(err) => err.to_message(),
            YankError::PasteSessionId(err) => err.to_message(),
        }
    }
}
