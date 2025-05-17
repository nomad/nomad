//! TODO: docs.

use ed::Context;
use ed::action::AsyncAction;
use ed::command::ToCompletionFn;
use ed::notify::{self, Name};

use crate::backend::{ActionForSelectedSession, CollabBackend};
use crate::collab::Collab;
use crate::project::{NoActiveSessionError, Projects};

/// An `Action` that pastes the [`SessionId`] of any active session to the
/// user's clipboard.
#[derive(cauchy::Clone)]
pub struct Yank<B: CollabBackend> {
    projects: Projects<B>,
}

impl<B: CollabBackend> AsyncAction<B> for Yank<B> {
    const NAME: Name = "yank";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut Context<B>,
    ) -> Result<(), YankError<B>> {
        let Some((_, session_id)) = self
            .projects
            .select(ActionForSelectedSession::CopySessionId, ctx)
            .await
            .map_err(YankError::NoActiveSession)?
        else {
            return Ok(());
        };

        B::copy_session_id(session_id, ctx)
            .await
            .map_err(YankError::PasteSessionId)
    }
}

/// The type of error that can occur when [`Yank`]ing fails.
pub enum YankError<B: CollabBackend> {
    /// TODO: docs.
    NoActiveSession(NoActiveSessionError<B>),

    /// TODO: docs.
    PasteSessionId(B::CopySessionIdError),
}

impl<B: CollabBackend> From<&Collab<B>> for Yank<B> {
    fn from(collab: &Collab<B>) -> Self {
        Self { projects: collab.projects.clone() }
    }
}

impl<B: CollabBackend> ToCompletionFn<B> for Yank<B> {
    fn to_completion_fn(&self) {}
}

impl<B: CollabBackend> notify::Error for YankError<B> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            YankError::NoActiveSession(err) => err.to_message(),
            YankError::PasteSessionId(err) => err.to_message(),
        }
    }
}
