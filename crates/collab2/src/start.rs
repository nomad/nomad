//! TODO: docs.

use core::fmt;
use core::marker::PhantomData;

use auth::AuthInfos;
use flume::Sender;
use nvimx2::action::AsyncAction;
use nvimx2::command::ToCompletionFn;
use nvimx2::notify::{self, Name};
use nvimx2::{AsyncCtx, Shared};

use crate::backend::{CollabBackend, StartArgs};
use crate::collab::Collab;
use crate::config::Config;
use crate::leave::StopChannels;
use crate::session::{NewSessionArgs, Session};
use crate::sessions::{OverlappingSessionError, Sessions};

/// The `Action` used to start a new collaborative editing session.
pub struct Start<B: CollabBackend> {
    auth_infos: Shared<Option<AuthInfos>>,
    config: Shared<Config>,
    sessions: Sessions,
    session_tx: Sender<Session<B>>,
    stop_channels: StopChannels,
}

impl<B: CollabBackend> AsyncAction<B> for Start<B> {
    const NAME: Name = "start";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), StartError<B>> {
        let auth_infos = self
            .auth_infos
            .with(|infos| infos.as_ref().cloned())
            .ok_or_else(StartError::user_not_logged_in)?;

        let buffer_id = ctx.with_ctx(|ctx| {
            ctx.current_buffer()
                .map(|buf| buf.id())
                .ok_or_else(StartError::no_buffer_focused)
        })?;

        let project_root = B::search_project_root(buffer_id, ctx)
            .await
            .map_err(StartError::SearchProjectRoot)?;

        if !B::confirm_start(&project_root, ctx).await {
            return Ok(());
        }

        let guard = self
            .sessions
            .start_guard(project_root)
            .map_err(StartError::OverlappingSession)?;

        let start_args = StartArgs {
            auth_infos: &auth_infos,
            project_root: guard.root(),
            server_address: &self.config.with(|c| c.server_address.clone()),
        };

        let start_infos = B::start_session(start_args, ctx)
            .await
            .map_err(StartError::StartSession)?;

        let replica =
            B::read_replica(start_infos.local_peer.id(), guard.root(), ctx)
                .await
                .map_err(StartError::ReadReplica)?;

        let session = Session::new(NewSessionArgs {
            _is_host: true,
            _local_peer: start_infos.local_peer,
            _remote_peers: start_infos.remote_peers,
            _replica: replica,
            server_rx: start_infos.server_rx,
            server_tx: start_infos.server_tx,
            session_guard: guard.into_active(start_infos.session_id),
            stop_rx: self.stop_channels.insert(start_infos.session_id),
        });

        self.session_tx
            .send_async(session)
            .await
            .map_err(|_| StartError::session_rx_dropped())
    }
}

/// The type of error that can occur when [`Start`]ing a new session fails.
#[derive(derive_more::Debug)]
#[debug(bound(B: CollabBackend))]
pub enum StartError<B: CollabBackend> {
    /// TODO: docs.
    NoBufferFocused(NoBufferFocusedError<B>),

    /// TODO: docs.
    OverlappingSession(OverlappingSessionError),

    /// TODO: docs.
    ReadReplica(B::ReadReplicaError),

    /// TODO: docs.
    SearchProjectRoot(B::SearchProjectRootError),

    /// TODO: docs.
    SessionRxDropped(SessionRxDroppedError<B>),

    /// TODO: docs.
    StartSession(B::StartSessionError),

    /// TODO: docs.
    UserNotLoggedIn(UserNotLoggedInError<B>),
}

/// TODO: docs.
pub struct NoBufferFocusedError<B>(PhantomData<B>);

/// TODO: docs.
pub struct SessionRxDroppedError<B>(PhantomData<B>);

/// TODO: docs.
pub struct UserNotLoggedInError<B>(PhantomData<B>);

impl<B: CollabBackend> Clone for Start<B> {
    fn clone(&self) -> Self {
        Self {
            auth_infos: self.auth_infos.clone(),
            config: self.config.clone(),
            stop_channels: self.stop_channels.clone(),
            sessions: self.sessions.clone(),
            session_tx: self.session_tx.clone(),
        }
    }
}

impl<B: CollabBackend> From<&Collab<B>> for Start<B> {
    fn from(collab: &Collab<B>) -> Self {
        Self {
            auth_infos: collab.auth_infos.clone(),
            config: collab.config.clone(),
            sessions: collab.sessions.clone(),
            session_tx: collab.session_tx.clone(),
            stop_channels: collab.stop_channels.clone(),
        }
    }
}

impl<B: CollabBackend> ToCompletionFn<B> for Start<B> {
    fn to_completion_fn(&self) {}
}

impl<B: CollabBackend> StartError<B> {
    /// Creates a new [`StartError::NoBufferFocused`] variant.
    pub fn no_buffer_focused() -> Self {
        Self::NoBufferFocused(NoBufferFocusedError(PhantomData))
    }

    /// Creates a new [`StartError::SessionRxDropped`] variant.
    pub fn session_rx_dropped() -> Self {
        Self::SessionRxDropped(SessionRxDroppedError(PhantomData))
    }

    /// Creates a new [`StartError::UserNotLoggedIn`] variant.
    pub fn user_not_logged_in() -> Self {
        Self::UserNotLoggedIn(UserNotLoggedInError(PhantomData))
    }
}

impl<B> PartialEq for StartError<B>
where
    B: CollabBackend,
    B::ReadReplicaError: PartialEq,
    B::SearchProjectRootError: PartialEq,
    B::StartSessionError: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        use StartError::*;

        match (self, other) {
            (NoBufferFocused(_), NoBufferFocused(_)) => true,
            (OverlappingSession(l), OverlappingSession(r)) => l == r,
            (ReadReplica(l), ReadReplica(r)) => l == r,
            (SearchProjectRoot(l), SearchProjectRoot(r)) => l == r,
            (SessionRxDropped(_), SessionRxDropped(_)) => true,
            (StartSession(l), StartSession(r)) => l == r,
            (UserNotLoggedIn(_), UserNotLoggedIn(_)) => true,
            _ => false,
        }
    }
}

impl<B: CollabBackend> notify::Error for StartError<B> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            StartError::NoBufferFocused(err) => err.to_message(),
            StartError::OverlappingSession(err) => err.to_message(),
            StartError::ReadReplica(err) => err.to_message(),
            StartError::SearchProjectRoot(err) => err.to_message(),
            StartError::SessionRxDropped(err) => err.to_message(),
            StartError::StartSession(err) => err.to_message(),
            StartError::UserNotLoggedIn(err) => err.to_message(),
        }
    }
}

impl<B> fmt::Debug for NoBufferFocusedError<B> {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl<B> fmt::Debug for SessionRxDroppedError<B> {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl<B> fmt::Debug for UserNotLoggedInError<B> {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl<B> notify::Error for NoBufferFocusedError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Off, notify::Message::new())
    }
}

impl<B> notify::Error for SessionRxDroppedError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Off, notify::Message::new())
    }
}

impl<B> notify::Error for UserNotLoggedInError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Off, notify::Message::new())
    }
}

#[cfg(feature = "neovim")]
mod neovim_error_impls {
    use nvimx2::neovim::Neovim;

    use super::*;

    impl notify::Error for NoBufferFocusedError<Neovim> {
        fn to_message(&self) -> (notify::Level, notify::Message) {
            let msg = "couldn't determine path to project root. Either move \
                       the cursor to a text buffer, or pass one explicitly";
            (notify::Level::Error, notify::Message::from_str(msg))
        }
    }

    impl notify::Error for UserNotLoggedInError<Neovim> {
        fn to_message(&self) -> (notify::Level, notify::Message) {
            let mut msg = notify::Message::from_str(
                "need to be logged in to collaborate. You can log in by \
                 executing ",
            );
            msg.push_expected(":Mad login");
            (notify::Level::Error, msg)
        }
    }
}
