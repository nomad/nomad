//! TODO: docs.

use std::collections::hash_map::Entry;

use ed::action::AsyncAction;
use ed::command::ToCompletionFn;
use ed::notify::Name;
use ed::{Context, Shared};
use flume::{Receiver, Sender};
use fxhash::FxHashMap;

use crate::backend::{ActionForSelectedSession, CollabBackend, SessionId};
use crate::collab::Collab;
use crate::project::{NoActiveSessionError, Projects};

/// TODO: docs.
#[derive(cauchy::Clone)]
pub struct Leave<B: CollabBackend> {
    channels: StopChannels<B>,
    projects: Projects<B>,
}

#[derive(cauchy::Clone, cauchy::Default)]
pub(crate) struct StopChannels<B: CollabBackend> {
    inner: Shared<FxHashMap<SessionId<B>, Sender<StopRequest>>>,
}

pub(crate) struct StopRequest {
    stopped_tx: Sender<()>,
}

impl<B: CollabBackend> AsyncAction<B> for Leave<B> {
    const NAME: Name = "leave";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut Context<B>,
    ) -> Result<(), NoActiveSessionError<B>> {
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

impl<B: CollabBackend> StopChannels<B> {
    #[track_caller]
    pub(crate) fn insert(
        &self,
        session_id: SessionId<B>,
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

    fn take(&self, session_id: SessionId<B>) -> Option<Sender<StopRequest>> {
        self.inner.with_mut(|inner| inner.remove(&session_id))
    }
}

impl StopRequest {
    pub(crate) fn send_stopped(self) {
        self.stopped_tx.send(()).expect("rx is still alive");
    }
}

impl<B: CollabBackend> From<&Collab<B>> for Leave<B> {
    fn from(collab: &Collab<B>) -> Self {
        Self {
            channels: collab.stop_channels.clone(),
            projects: collab.projects.clone(),
        }
    }
}

impl<B: CollabBackend> ToCompletionFn<B> for Leave<B> {
    fn to_completion_fn(&self) {}
}
