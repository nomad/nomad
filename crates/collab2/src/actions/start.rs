use collab_server::message::Message;
use futures_util::StreamExt;
use nomad::ctx::NeovimCtx;
use nomad::{action_name, ActionName, AsyncAction, Shared};

use super::UserBusyError;
use crate::session::Session;
use crate::session_status::SessionStatus;
use crate::Collab;

#[derive(Clone)]
pub(crate) struct Start {
    session_status: Shared<SessionStatus>,
}

impl Start {
    pub(crate) fn new(session_status: Shared<SessionStatus>) -> Self {
        Self { session_status }
    }
}

impl AsyncAction for Start {
    const NAME: ActionName = action_name!("start");
    type Args = ();
    type Docs = ();
    type Module = Collab;

    async fn execute(
        &mut self,
        _: Self::Args,
        ctx: NeovimCtx<'_>,
    ) -> Result<(), UserBusyError> {
        match self.session_status.with(|s| UserBusyError::try_from(s)).ok() {
            Some(err) => return Err(err),
            _ => self.session_status.set(SessionStatus::Starting),
        }

        let mut session = Session::start().await;
        self.session_status.set(SessionStatus::InSession(session.project()));
        ctx.spawn(async move {
            let (tx, rx) = flume::unbounded::<Message>();
            let tx = tx.into_sink::<'static>();
            let rx = rx
                .into_stream::<'static>()
                .map(Ok::<_, core::convert::Infallible>);
            let _err = session.run(tx, rx).await;
        });

        Ok(())
    }

    fn docs(&self) -> Self::Docs {}
}
