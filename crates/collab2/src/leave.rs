use std::collections::hash_map::Entry;

use collab_server::SessionId;
use flume::{Receiver, Sender};
use fxhash::FxHashMap;
use nvimx2::action::AsyncAction;
use nvimx2::command::ToCompletionFn;
use nvimx2::notify::Name;
use nvimx2::{AsyncCtx, Shared};

use crate::backend::CollabBackend;
use crate::collab::Collab;
use crate::yank::{NoActiveSessionError, SessionSelector};

/// TODO: docs.
#[derive(Clone)]
pub struct Leave {
    channels: LeaveChannels,
    session_selector: SessionSelector,
}

#[derive(Clone, Default)]
pub(crate) struct LeaveChannels {
    inner: Shared<FxHashMap<SessionId, Sender<LeaveSession>>>,
}

pub(crate) struct LeaveSession;

impl<B: CollabBackend> AsyncAction<B> for Leave {
    const NAME: Name = "leave";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), NoActiveSessionError<B>> {
        if let Some((_, id)) = self.session_selector.select(ctx).await? {
            if let Some(sender) = self.channels.take(id) {
                let _ = sender.send_async(LeaveSession).await;
            }
        }

        Ok(())
    }
}

impl LeaveChannels {
    #[track_caller]
    pub(crate) fn insert(
        &self,
        session_id: SessionId,
    ) -> Receiver<LeaveSession> {
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

    fn take(&self, session_id: SessionId) -> Option<Sender<LeaveSession>> {
        self.inner.with_mut(|inner| inner.remove(&session_id))
    }
}

impl<B: CollabBackend> From<&Collab<B>> for Leave {
    fn from(collab: &Collab<B>) -> Self {
        Self {
            channels: collab.leave_channels.clone(),
            session_selector: SessionSelector::new(collab.sessions.clone()),
        }
    }
}

impl<B: CollabBackend> ToCompletionFn<B> for Leave {
    fn to_completion_fn(&self) {}
}
