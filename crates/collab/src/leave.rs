//! TODO: docs.

use std::collections::hash_map::Entry;

use ed::action::AsyncAction;
use ed::command::ToCompletionFn;
use ed::notify::Name;
use ed::{Context, Shared};
use flume::{Receiver, Sender};
use fxhash::FxHashMap;

use crate::collab::Collab;
use crate::editors::{ActionForSelectedSession, CollabEditor, SessionId};
use crate::project::{NoActiveSessionError, Projects};

/// TODO: docs.
#[derive(cauchy::Clone)]
pub struct Leave<Ed: CollabEditor> {
    channels: StopChannels<Ed>,
    projects: Projects<Ed>,
}

#[derive(cauchy::Clone, cauchy::Default)]
pub(crate) struct StopChannels<Ed: CollabEditor> {
    inner: Shared<FxHashMap<SessionId<Ed>, Sender<StopRequest>>>,
}

pub(crate) struct StopRequest {
    stopped_tx: Sender<()>,
}

impl<Ed: CollabEditor> AsyncAction<Ed> for Leave<Ed> {
    const NAME: Name = "leave";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut Context<Ed>,
    ) -> Result<(), NoActiveSessionError<Ed>> {
        let Some(stop_sender) = self
            .projects
            .select(ActionForSelectedSession::Leave, ctx)
            .await?
            .and_then(|(_, session_id)| self.channels.take(session_id))
        else {
            return Ok(());
        };

        let (stopped_tx, stopped_rx) = flume::bounded(1);

        // Wait for the session to receive the stop request and actually stop.
        if stop_sender.send_async(StopRequest { stopped_tx }).await.is_ok() {
            let _ = stopped_rx.recv_async().await;
        }

        Ok(())
    }
}

impl<Ed: CollabEditor> StopChannels<Ed> {
    #[track_caller]
    pub(crate) fn insert(
        &self,
        session_id: SessionId<Ed>,
    ) -> Receiver<StopRequest> {
        let (tx, rx) = flume::bounded(1);
        self.inner.with_mut(move |inner| match inner.entry(session_id) {
            Entry::Vacant(vacant) => {
                vacant.insert(tx);
            },
            Entry::Occupied(_) => {
                panic!("already have a sender for {session_id:?}")
            },
        });
        rx
    }

    fn take(&self, session_id: SessionId<Ed>) -> Option<Sender<StopRequest>> {
        self.inner.with_mut(|inner| inner.remove(&session_id))
    }
}

impl StopRequest {
    pub(crate) fn send_stopped(self) {
        self.stopped_tx.send(()).expect("rx is still alive");
    }
}

impl<Ed: CollabEditor> From<&Collab<Ed>> for Leave<Ed> {
    fn from(collab: &Collab<Ed>) -> Self {
        Self {
            channels: collab.stop_channels.clone(),
            projects: collab.projects.clone(),
        }
    }
}

impl<Ed: CollabEditor> ToCompletionFn<Ed> for Leave<Ed> {
    fn to_completion_fn(&self) {}
}
