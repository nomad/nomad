//! TODO: docs.

use core::cell::Cell;
use core::pin::Pin;
use core::task::{self, Poll, ready};
use std::collections::{VecDeque, hash_map};
use std::io;
use std::rc::Rc;

use abs_path::{AbsPathBuf, NodeName};
use collab_server::client as collab_client;
use collab_types::{Message, Peer, PeerId};
use editor::{Access, Context, Shared};
use futures_util::sink::{Sink, SinkExt};
use futures_util::stream::{FusedStream, Stream, StreamExt};
use futures_util::{FutureExt, select_biased};
use fxhash::FxHashMap;
use smallvec::SmallVec;

use crate::editors::ActionForSelectedSession;
use crate::event_stream::{EventError, EventStream};
use crate::leave::StopRequest;
use crate::peers::RemotePeers;
use crate::project::{IntegrateError, Project, SynchronizeError};
use crate::{CollabEditor, SessionId, pausable_stream};

/// The type-erased version of the async callbacks given to
/// [`ProjectAccess::with()`].
type ProjectAccessCallback<Ed> = Box<
    dyn for<'a> FnOnce(
        &'a Project<Ed>,
        &'a mut Context<Ed>,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>>,
>;

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

    /// TODO: docs.
    pub(crate) project_access: ProjectAccess<Ed>,

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
pub(crate) struct Session<Ed: CollabEditor, Tx, Rx> {
    /// TODO: docs.
    pub(crate) event_stream: EventStream<Ed>,

    /// TODO: docs.
    pub(crate) message_rx: Rx,

    /// TODO: docs.
    pub(crate) message_tx: Tx,

    /// TODO: docs.
    pub(crate) project_access: ProjectAccess<Ed>,

    /// TODO: docs.
    pub(crate) project: Project<Ed>,

    /// TODO: docs.
    pub(crate) stop_rx: flume::Receiver<StopRequest>,

    /// TODO: docs.
    pub(crate) remove_on_drop: RemoveOnDrop<Ed>,
}

/// TODO: docs.
#[derive(cauchy::Debug, cauchy::Default, cauchy::Clone)]
pub(crate) struct ProjectAccess<Ed: CollabEditor> {
    #[debug(skip)]
    callbacks: Shared<VecDeque<ProjectAccessCallback<Ed>>>,
    #[debug(skip)]
    event: Rc<event_listener::Event>,
    event_loop_has_started: Rc<Cell<bool>>,
}

/// TODO: docs.
pub(crate) struct RemoveOnDrop<Ed: CollabEditor> {
    sessions: Sessions<Ed>,
    session_id: SessionId<Ed>,
}

/// TODO: docs.
#[derive(Debug, derive_more::Display, cauchy::Error, PartialEq, Eq)]
#[display("There's no active collaborative editing session")]
pub(crate) struct NoActiveSessionError;

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

    pub(crate) fn find(
        &self,
        mut fun: impl FnMut(&SessionInfos<Ed>) -> bool,
    ) -> Option<SessionInfos<Ed>> {
        self.with(|map| {
            map.values().find_map(|infos| fun(infos).then(|| infos.clone()))
        })
    }

    /// Calls the given function on the infos of all the current sessions.
    pub(crate) fn for_each(&self, fun: impl FnMut(&SessionInfos<Ed>)) {
        self.with(|map| map.values().for_each(fun))
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
            project_access,
            stop_rx,
            ..
        } = self;

        let mut callback_stream = project_access.callback_stream();
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
                callback = callback_stream.select_next_some() => {
                    callback(project, ctx).await;
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

impl<Ed: CollabEditor> ProjectAccess<Ed> {
    /// TODO: docs.
    pub(crate) async fn with<R: 'static>(
        &self,
        fun: impl AsyncFnOnce(&Project<Ed>, &mut Context<Ed>) -> R + 'static,
    ) -> Option<R> {
        let (tx, rx) = flume::bounded(1);

        let callback: ProjectAccessCallback<Ed> =
            Box::new(move |project, ctx| {
                Box::pin(async move {
                    let _ = tx.send(fun(project, ctx).await);
                })
            });

        self.callbacks.with_mut(|queue| queue.push_back(callback));

        let num_notified = self.event.notify(1);

        debug_assert!(
            num_notified <= 1,
            "only the session's event loop should be notified"
        );

        // If no one was notified, then either the session event loop hasn't
        // started yet, or it has already ended. In the latter case, we remove
        // the callback we just pushed and return early.
        if num_notified == 0 && self.event_loop_has_started.get() {
            let _ = self
                .callbacks
                .with_mut(|queue| queue.pop_back().expect("just pushed"));
            return None;
        }

        rx.into_recv_async().await.ok()
    }

    fn callback_stream(
        &mut self,
    ) -> impl FusedStream<Item = ProjectAccessCallback<Ed>> {
        pin_project_lite::pin_project! {
            struct CallbackStream<'this, Ed: CollabEditor> {
                access: &'this ProjectAccess<Ed>,
                #[pin]
                listener: event_listener::EventListener,
            }
        }

        impl<Ed: CollabEditor> Stream for CallbackStream<'_, Ed> {
            type Item = ProjectAccessCallback<Ed>;

            fn poll_next(
                self: Pin<&mut Self>,
                ctx: &mut task::Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                let mut this = self.project();
                let ProjectAccess { callbacks, event, .. } = this.access;

                loop {
                    match callbacks.with_mut(|queue| queue.pop_front()) {
                        Some(callback) => return Poll::Ready(Some(callback)),
                        None => {
                            ready!(this.listener.as_mut().poll(ctx));
                            this.listener.as_mut().set(event.listen());
                        },
                    }
                }
            }
        }

        impl<Ed: CollabEditor> FusedStream for CallbackStream<'_, Ed> {
            fn is_terminated(&self) -> bool {
                false
            }
        }

        self.event_loop_has_started.set(true);

        CallbackStream { access: self, listener: self.event.listen() }
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

impl<Ed: CollabEditor> Drop for RemoveOnDrop<Ed> {
    fn drop(&mut self) {
        assert!(self.sessions.remove(self.session_id));
    }
}
