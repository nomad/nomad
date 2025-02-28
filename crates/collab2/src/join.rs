//! TODO: docs.

use core::marker::PhantomData;

use auth::AuthInfos;
use collab_server::SessionId;
use nvimx2::action::AsyncAction;
use nvimx2::command::{Parse, ToCompletionFn};
use nvimx2::notify::Name;
use nvimx2::{AsyncCtx, Shared, notify};

use crate::backend::{CollabBackend, JoinArgs};
use crate::collab::Collab;
use crate::config::Config;
use crate::leave::StopChannels;
use crate::session::Session;
use crate::sessions::{OverlappingSessionError, Sessions};
use crate::start::{SessionRxDroppedError, UserNotLoggedInError};

/// The `Action` used to join an existing collaborative editing session.
pub struct Join<B: CollabBackend> {
    auth_infos: Shared<Option<AuthInfos>>,
    config: Shared<Config>,
    sessions: Sessions,
    session_tx: flume::Sender<Session<B>>,
    stop_channels: StopChannels,
}

impl<B: CollabBackend> AsyncAction<B> for Join<B> {
    const NAME: Name = "join";

    type Args = Parse<SessionId>;

    async fn call(
        &mut self,
        args: Self::Args,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), JoinError<B>> {
        let auth_infos = self
            .auth_infos
            .with(|infos| infos.as_ref().cloned())
            .ok_or_else(JoinError::user_not_logged_in)?;

        let join_args = JoinArgs {
            auth_infos: &auth_infos,
            session_id: args.into_inner(),
            server_address: &self.config.with(|c| c.server_address.clone()),
        };

        let _join_infos = B::join_session(join_args, ctx)
            .await
            .map_err(JoinError::JoinSession)?;

        todo!();
    }
}

/// The type of error that can occur when [`Join`]ing a session fails.
#[derive(derive_more::Debug)]
#[debug(bound(B: CollabBackend))]
pub enum JoinError<B: CollabBackend> {
    /// TODO: docs.
    JoinSession(B::JoinSessionError),

    /// TODO: docs.
    OverlappingSession(OverlappingSessionError),

    /// TODO: docs.
    SessionRxDropped(SessionRxDroppedError<B>),

    /// TODO: docs.
    UserNotLoggedIn(UserNotLoggedInError<B>),
}

impl<B: CollabBackend> Clone for Join<B> {
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

impl<B: CollabBackend> From<&Collab<B>> for Join<B> {
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

impl<B: CollabBackend> ToCompletionFn<B> for Join<B> {
    fn to_completion_fn(&self) {}
}

impl<B: CollabBackend> JoinError<B> {
    /// Creates a new [`JoinError::SessionRxDropped`] variant.
    pub fn session_rx_dropped() -> Self {
        Self::SessionRxDropped(SessionRxDroppedError(PhantomData))
    }

    /// Creates a new [`JoinError::UserNotLoggedIn`] variant.
    pub fn user_not_logged_in() -> Self {
        Self::UserNotLoggedIn(UserNotLoggedInError(PhantomData))
    }
}

impl<B> PartialEq for JoinError<B>
where
    B: CollabBackend,
    B::JoinSessionError: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        use JoinError::*;

        match (self, other) {
            (JoinSession(l), JoinSession(r)) => l == r,
            (OverlappingSession(l), OverlappingSession(r)) => l == r,
            (SessionRxDropped(_), SessionRxDropped(_)) => true,
            (UserNotLoggedIn(_), UserNotLoggedIn(_)) => true,
            _ => false,
        }
    }
}

impl<B: CollabBackend> notify::Error for JoinError<B> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            Self::JoinSession(err) => err.to_message(),
            Self::OverlappingSession(err) => err.to_message(),
            Self::SessionRxDropped(err) => err.to_message(),
            Self::UserNotLoggedIn(err) => err.to_message(),
        }
    }
}
