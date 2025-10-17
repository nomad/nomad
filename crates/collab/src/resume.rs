//! TODO: docs.

use editor::Context;
use editor::command::ToCompletionFn;
use editor::module::AsyncAction;

use crate::collab::Collab;
use crate::editors::{ActionForSelectedSession, CollabEditor};
use crate::session::{NoActiveSessionError, SessionInfos, Sessions};

/// TODO: docs.
#[derive(cauchy::Clone)]
pub struct Resume<Ed: CollabEditor> {
    sessions: Sessions<Ed>,
}

impl<Ed: CollabEditor> Resume<Ed> {
    pub(crate) async fn call_inner(
        &self,
        ctx: &mut Context<Ed>,
    ) -> Result<(), ResumeError<Ed>> {
        let Some(session_infos) = self
            .sessions
            .select(ActionForSelectedSession::Resume, ctx)
            .await?
            .and_then(|(_, session_id)| self.sessions.get(session_id))
        else {
            return Ok(());
        };

        if session_infos.rx_remote.resume() {
            Ok(())
        } else {
            Err(ResumeError::SessionIsNotPaused(session_infos))
        }
    }
}

impl<Ed: CollabEditor> AsyncAction<Ed> for Resume<Ed> {
    const NAME: &str = "leave";

    type Args = ();

    async fn call(&mut self, _: Self::Args, ctx: &mut Context<Ed>) {
        if let Err(err) = self.call_inner(ctx).await {
            Ed::on_resume_error(err, ctx);
        }
    }
}

/// The type of error that can occur when [`Resume`]ing fails.
#[derive(
    cauchy::Debug, derive_more::Display, cauchy::Error, cauchy::PartialEq,
)]
pub enum ResumeError<Ed: CollabEditor> {
    /// There are no active sessions to resume.
    #[display("{}", NoActiveSessionError)]
    NoActiveSession,

    /// The session is already resumed.
    #[display("The session is not paused")]
    SessionIsNotPaused(SessionInfos<Ed>),
}

impl<Ed: CollabEditor> From<&Collab<Ed>> for Resume<Ed> {
    fn from(collab: &Collab<Ed>) -> Self {
        Self { sessions: collab.sessions.clone() }
    }
}

impl<Ed: CollabEditor> ToCompletionFn<Ed> for Resume<Ed> {
    fn to_completion_fn(&self) {}
}

impl<Ed: CollabEditor> From<NoActiveSessionError> for ResumeError<Ed> {
    fn from(_: NoActiveSessionError) -> Self {
        Self::NoActiveSession
    }
}
