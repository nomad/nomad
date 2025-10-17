//! TODO: docs.

use std::collections::hash_map;
use std::io;

use abs_path::{AbsPathBuf, NodeName};
use collab_server::client as collab_client;
use collab_types::{Message, Peer, PeerId};
use editor::{Access, Context, Shared};
use futures_util::sink::{Sink, SinkExt};
use futures_util::stream::{FusedStream, StreamExt};
use futures_util::{FutureExt, select_biased};
use fxhash::FxHashMap;
use smallvec::SmallVec;

use crate::editors::ActionForSelectedSession;
use crate::event_stream::{EventError, EventStream};
use crate::leave::StopRequest;
use crate::project::{IntegrateError, Project, SynchronizeError};
use crate::{CollabEditor, SessionId, pausable_stream};

/// TODO: docs.
#[derive(cauchy::Debug, cauchy::Default, cauchy::Clone)]
pub struct Sessions<Ed: CollabEditor> {
    inner: Shared<FxHashMap<SessionId<Ed>, SessionInfos<Ed>>>,
}

/// TODO: docs.
#[derive(cauchy::Debug, cauchy::Clone)]
#[allow(dead_code)]
pub struct SessionInfos<Ed: CollabEditor> {
    /// The [`PeerId`] of the host of the session.
    pub(crate) host_id: PeerId,

    /// TODO: docs..
    pub(crate) local_peer: Peer,

    /// TODO: docs..
    pub(crate) remote_peers: RemotePeers,

    /// The remote used to pause/resume receiving [`Message`]s.
    pub(crate) rx_remote: pausable_stream::Remote,

    /// The path to the root of the project.
    pub(crate) project_root_path: AbsPathBuf,

    /// The ID of the session.
    pub(crate) session_id: SessionId<Ed>,
}

/// TODO: docs.
#[derive(cauchy::Debug, derive_more::Display, cauchy::Error, cauchy::From)]
#[display("{_0}")]
pub enum SessionError<Ed: CollabEditor> {
    /// TODO: docs.
    Event(#[from] EventError<Ed>),

    /// TODO: docs.
    Integrate(#[from] IntegrateError<Ed>),

    /// TODO: docs.
    MessageRx(#[from] collab_client::ReceiveError),

    /// TODO: docs.
    MessageTx(#[from] io::Error),

    /// TODO: docs.
    Synchronize(#[from] SynchronizeError<Ed>),
}

/// TODO: docs.
#[derive(Debug, derive_more::Display, cauchy::Error, PartialEq, Eq)]
#[display("there's no active collaborative editing session")]
pub struct NoActiveSessionError;

/// TODO: docs.
pub(crate) struct Session<Ed: CollabEditor, Tx, Rx> {
    /// TODO: docs.
    pub(crate) event_stream: EventStream<Ed>,

    /// TODO: docs.
    pub(crate) message_rx: Rx,

    /// TODO: docs.
    pub(crate) message_tx: Tx,

    /// TODO: docs.
    pub(crate) project: Project<Ed>,

    /// TODO: docs.
    pub(crate) stop_rx: flume::Receiver<StopRequest>,

    /// TODO: docs.
    pub(crate) remove_on_drop: RemoveOnDrop<Ed>,
}

/// TODO: docs.
#[derive(Debug, Default, Clone)]
pub struct RemotePeers {
    /// A map of all the peers currently in the session.
    ///
    /// It also includes the local peer, so it's guaranteed to never be empty.
    inner: Shared<FxHashMap<PeerId, Peer>>,
}

/// TODO: docs.
pub(crate) struct RemoveOnDrop<Ed: CollabEditor> {
    sessions: Sessions<Ed>,
    session_id: SessionId<Ed>,
}

/// Represents the reasons a session can end, excluding [errors](SessionError).
enum SessionEndReason {
    /// The message receiver was exhausted.
    MessageReceiverExhausted,

    /// The [`Leave`](crate::leave::Leave) action was invoked.
    UserLeft,
}

impl<Ed: CollabEditor> Sessions<Ed> {
    /// Returns the infos for the session with the given ID, if any.
    pub fn get(&self, session_id: SessionId<Ed>) -> Option<SessionInfos<Ed>> {
        self.with_session(session_id, |infos| infos.cloned())
    }

    /// Calls the given function on the infos of all the current sessions.
    pub(crate) fn for_each(&self, mut fun: impl FnMut(&SessionInfos<Ed>)) {
        self.with(|map| {
            for infos in map.values() {
                fun(infos);
            }
        });
    }

    /// Inserts the given infos.
    ///
    /// # Panics
    ///
    /// Panics if there are already infos with the same session ID.
    #[track_caller]
    pub(crate) fn insert(&self, infos: SessionInfos<Ed>) -> RemoveOnDrop<Ed> {
        let session_id = infos.session_id;

        self.inner.with_mut(|inner| match inner.entry(session_id) {
            hash_map::Entry::Vacant(vacant) => {
                vacant.insert(infos);
            },
            hash_map::Entry::Occupied(_) => {
                panic!("already have infos for {:?}", infos.session_id)
            },
        });

        RemoveOnDrop { sessions: self.clone(), session_id }
    }

    pub(crate) async fn select(
        &self,
        action: ActionForSelectedSession,
        ctx: &mut Context<Ed>,
    ) -> Result<Option<(AbsPathBuf, SessionId<Ed>)>, NoActiveSessionError>
    {
        let active_sessions = self.inner.with(|map| {
            map.iter()
                .map(|(session_id, infos)| {
                    (infos.project_root_path.clone(), *session_id)
                })
                .collect::<SmallVec<[_; 1]>>()
        });

        let session = match &*active_sessions {
            [] => return Err(NoActiveSessionError),
            [single] => single,
            sessions => {
                match Ed::select_session(sessions, action, ctx).await {
                    Some(session) => session,
                    None => return Ok(None),
                }
            },
        };

        Ok(Some(session.clone()))
    }

    fn remove(&self, session_id: SessionId<Ed>) -> bool {
        self.inner.with_mut(|inner| inner.remove(&session_id).is_some())
    }

    /// Runs the given function with the infos for the session with the given
    /// ID, if any.
    fn with_session<R>(
        &self,
        session_id: SessionId<Ed>,
        fun: impl FnOnce(Option<&SessionInfos<Ed>>) -> R,
    ) -> R {
        self.inner.with(|inner| fun(inner.get(&session_id)))
    }
}

impl<Ed: CollabEditor> SessionInfos<Ed> {
    /// Returns the session's ID.
    pub fn id(&self) -> SessionId<Ed> {
        self.session_id
    }

    /// Returns the name of the project tracked by this session.
    pub fn proj_name(&self) -> &NodeName {
        self.project_root_path
            .node_name()
            .expect("project can't be rooted at fs root")
    }
}

impl RemotePeers {
    /// Calls the given function on all the remote peers.
    pub(crate) fn for_each(&self, mut fun: impl FnMut(&Peer)) {
        self.with(|map| {
            for peer in map.values() {
                fun(peer);
            }
        });
    }

    pub(crate) fn get(&self, peer_id: PeerId) -> Option<Peer> {
        self.inner.with(|inner| inner.get(&peer_id).cloned())
    }

    #[track_caller]
    pub(crate) fn insert(&self, peer: Peer) {
        self.inner.with_mut(|inner| match inner.entry(peer.id) {
            hash_map::Entry::Vacant(vacant) => {
                vacant.insert(peer);
            },
            hash_map::Entry::Occupied(occupied) => {
                panic!(
                    "peer with ID {:?} already exists: {:?}",
                    peer.id,
                    occupied.get()
                )
            },
        });
    }

    #[track_caller]
    pub(crate) fn remove(&self, peer_id: PeerId) -> Peer {
        self.inner.with_mut(|inner| match inner.remove(&peer_id) {
            Some(peer) => peer,
            None => panic!("no peer with ID {:?} exists", peer_id),
        })
    }
}

impl<Ed, Tx, Rx> Session<Ed, Tx, Rx>
where
    Ed: CollabEditor,
    Tx: Sink<Message, Error = io::Error> + Unpin,
    Rx: FusedStream<Item = Result<Message, collab_client::ReceiveError>>
        + Unpin,
{
    pub(crate) async fn run(mut self, ctx: &mut Context<Ed>) {
        match self.run_event_loop(ctx).await {
            Ok(SessionEndReason::MessageReceiverExhausted) => {
                self.with_infos(|infos| Ed::on_session_ended(infos, ctx));
            },
            Ok(SessionEndReason::UserLeft) => {
                self.with_infos(|infos| Ed::on_session_left(infos, ctx));
            },
            Err(err) => Ed::on_session_error(err, ctx),
        }
    }

    /// Runs the session's event loop until the session ends or an error
    /// occurs.
    async fn run_event_loop(
        &mut self,
        ctx: &mut Context<Ed>,
    ) -> Result<SessionEndReason, SessionError<Ed>> {
        let Self {
            event_stream,
            message_rx,
            message_tx,
            project,
            stop_rx,
            ..
        } = self;

        let mut stop_stream = stop_rx.stream();

        loop {
            select_biased! {
                event_res = event_stream.next(ctx).fuse() => {
                    if let Some(message) =
                        project.synchronize(event_res?, ctx).await?
                    {
                        message_tx.send(message).await?;
                    }
                },
                maybe_message_res = message_rx.next() => {
                    let Some(message_res) = maybe_message_res else {
                        return Ok(SessionEndReason::MessageReceiverExhausted);
                    };

                    let message = message_res?;

                    for message in project.integrate(message, ctx).await? {
                        message_tx.send(message).await?;
                    }
                },
                stop_request = stop_stream.select_next_some() => {
                    stop_request.send_stopped();
                    return Ok(SessionEndReason::UserLeft);
                },
            }
        }
    }

    /// Calls the given function with the infos for this session.
    fn with_infos<R>(&self, f: impl FnOnce(&SessionInfos<Ed>) -> R) -> R {
        self.remove_on_drop.sessions.with_session(
            self.remove_on_drop.session_id,
            |maybe_infos| {
                f(maybe_infos.expect("session is alive, so infos must exist"))
            },
        )
    }
}

impl<Ed: CollabEditor> Access<FxHashMap<SessionId<Ed>, SessionInfos<Ed>>>
    for Sessions<Ed>
{
    fn with<R>(
        &self,
        fun: impl FnOnce(&FxHashMap<SessionId<Ed>, SessionInfos<Ed>>) -> R,
    ) -> R {
        self.inner.with(fun)
    }
}

impl Access<FxHashMap<PeerId, Peer>> for RemotePeers {
    fn with<R>(&self, fun: impl FnOnce(&FxHashMap<PeerId, Peer>) -> R) -> R {
        self.inner.with(fun)
    }
}

impl From<collab_types::Peers> for RemotePeers {
    fn from(peers: collab_types::Peers) -> Self {
        peers.into_iter().collect::<Self>()
    }
}

impl FromIterator<collab_types::Peer> for RemotePeers {
    fn from_iter<T: IntoIterator<Item = collab_types::Peer>>(iter: T) -> Self {
        Self {
            inner: Shared::new(
                iter.into_iter().map(|peer| (peer.id, peer)).collect(),
            ),
        }
    }
}

impl<Ed: CollabEditor> Drop for RemoveOnDrop<Ed> {
    fn drop(&mut self) {
        assert!(self.sessions.remove(self.session_id));
    }
}
