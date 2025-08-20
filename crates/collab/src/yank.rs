//! TODO: docs.

use editor::Context;
use editor::action::AsyncAction;
use editor::command::ToCompletionFn;

use crate::collab::Collab;
use crate::editors::{ActionForSelectedSession, CollabEditor};
use crate::project::{NoActiveSessionError, Projects};

/// An `Action` that pastes the [`SessionId`](crate::editors::SessionId) of any
/// active session to the user's clipboard.
#[derive(cauchy::Clone)]
pub struct Yank<Ed: CollabEditor> {
    projects: Projects<Ed>,
}

impl<Ed: CollabEditor> Yank<Ed> {
    pub(crate) async fn call_inner(
        &self,
        ctx: &mut Context<Ed>,
    ) -> Result<(), YankError<Ed>> {
        let Some((_, session_id)) = self
            .projects
            .select(ActionForSelectedSession::CopySessionId, ctx)
            .await?
        else {
            return Ok(());
        };

        Ed::copy_session_id(session_id, ctx)
            .await
            .map_err(YankError::PasteSessionId)
    }
}

impl<Ed: CollabEditor> AsyncAction<Ed> for Yank<Ed> {
    const NAME: &str = "yank";

    type Args = ();

    async fn call(&mut self, _: Self::Args, ctx: &mut Context<Ed>) {
        if let Err(err) = self.call_inner(ctx).await {
            Ed::on_yank_error(err, ctx);
        }
    }
}

/// The type of error that can occur when [`Yank`]ing fails.
#[derive(
    cauchy::Debug, derive_more::Display, cauchy::Error, cauchy::PartialEq,
)]
pub enum YankError<Ed: CollabEditor> {
    /// TODO: docs.
    #[display("{}", NoActiveSessionError)]
    NoActiveSession,

    /// TODO: docs.
    #[display("{_0}")]
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

impl<Ed: CollabEditor> From<NoActiveSessionError> for YankError<Ed> {
    fn from(_: NoActiveSessionError) -> Self {
        Self::NoActiveSession
    }
}
