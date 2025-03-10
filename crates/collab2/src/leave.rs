use std::collections::hash_map::Entry;

use flume::{Receiver, Sender};
use fxhash::FxHashMap;
use nvimx2::action::AsyncAction;
use nvimx2::command::ToCompletionFn;
use nvimx2::notify::Name;
use nvimx2::{AsyncCtx, Shared};

use crate::backend::{ActionForSelectedSession, CollabBackend};
use crate::collab::Collab;
use crate::project::{NoActiveSessionError, Projects};

/// TODO: docs.
pub struct Leave<B: CollabBackend> {
    channels: StopChannels<B>,
    projects: Projects<B>,
}

pub(crate) struct StopChannels<B: CollabBackend> {
    inner: Shared<FxHashMap<B::SessionId, Sender<StopSession>>>,
}

pub(crate) struct StopSession;

impl<B: CollabBackend> AsyncAction<B> for Leave<B> {
    const NAME: Name = "leave";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), NoActiveSessionError<B>> {
        if let Some((_, id)) =
            self.projects.select(ActionForSelectedSession::Leave, ctx).await?
        {
            if let Some(sender) = self.channels.take(id) {
                let _ = sender.send_async(StopSession).await;
            }
        }

        Ok(())
    }
}

impl<B: CollabBackend> StopChannels<B> {
    #[track_caller]
    pub(crate) fn insert(
        &self,
        session_id: B::SessionId,
    ) -> Receiver<StopSession> {
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

    fn take(&self, session_id: B::SessionId) -> Option<Sender<StopSession>> {
        self.inner.with_mut(|inner| inner.remove(&session_id))
    }
}

impl<B: CollabBackend> Clone for Leave<B> {
    fn clone(&self) -> Self {
        Self {
            channels: self.channels.clone(),
            projects: self.projects.clone(),
        }
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

impl<B: CollabBackend> Default for StopChannels<B> {
    fn default() -> Self {
        Self { inner: Default::default() }
    }
}

impl<B: CollabBackend> Clone for StopChannels<B> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}
