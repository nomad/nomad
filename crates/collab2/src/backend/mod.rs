//! TODO: docs.

#[cfg(feature = "neovim")]
mod neovim;
#[cfg(feature = "test")]
pub mod test;

use core::error::Error as StdError;
use core::fmt::Debug;
use core::hash::Hash;
use core::str::FromStr;

use collab_server::message::{Message, Peer, Peers};
use eerie::PeerId;
use futures_util::{Sink, Stream};
use nvimx2::backend::{Backend, Buffer};
use nvimx2::fs::{self, AbsPath, AbsPathBuf, FsNodeNameBuf};
use nvimx2::{AsyncCtx, notify};

use crate::config;

/// A [`Backend`] subtrait defining additional capabilities needed by the
/// actions in this crate.
pub trait CollabBackend: Backend {
    /// TODO: docs.
    type ServerRx: Stream<Item = Result<Message, Self::ServerRxError>> + Unpin;

    /// TODO: docs.
    type ServerTx: Sink<Message, Error = Self::ServerTxError> + Unpin;

    /// TODO: docs.
    type SessionId: Debug
        + Copy
        + FromStr<Err: StdError>
        + Eq
        + Hash
        + serde::de::DeserializeOwned;

    /// The type of error returned by
    /// [`copy_session_id`](CollabBackend::copy_session_id).
    type CopySessionIdError: Debug + notify::Error;

    /// The type of error returned by
    /// [`default_dir_for_remote_projects`](CollabBackend::default_dir_for_remote_projects).
    type DefaultDirForRemoteProjectsError: Debug + notify::Error;

    /// The type of error returned by [`home_dir`](CollabBackend::home_dir).
    type HomeDirError: Debug + notify::Error;

    /// The type of error returned by
    /// [`join_session`](CollabBackend::join_session).
    type JoinSessionError: Debug + notify::Error;

    /// The type of error returned by [`lsp_root`](CollabBackend::lsp_root).
    type LspRootError: Debug + notify::Error;

    /// TODO: docs.
    type ServerTxError: Debug + notify::Error;

    /// TODO: docs.
    type ServerRxError: Debug + notify::Error;

    /// The type of error returned by
    /// [`start_session`](CollabBackend::start_session).
    type StartSessionError: Debug + notify::Error;

    /// Asks the user to confirm starting a new collaborative editing session
    /// rooted at the given path.
    fn confirm_start(
        project_root: &AbsPath,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = bool>;

    /// Copies the given [`SessionId`](Self::SessionId) to the user's clipboard.
    fn copy_session_id(
        session_id: Self::SessionId,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Result<(), Self::CopySessionIdError>>;

    /// TODO: docs.
    fn default_dir_for_remote_projects(
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<
        Output = Result<AbsPathBuf, Self::DefaultDirForRemoteProjectsError>,
    >;

    /// Returns the absolute path to the user's home directory.
    fn home_dir(
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Result<AbsPathBuf, Self::HomeDirError>>;

    /// TODO: docs.
    fn join_session(
        args: JoinArgs<'_, Self>,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Result<SessionInfos<Self>, Self::JoinSessionError>>;

    /// Returns the path to the root of the workspace containing the buffer
    /// with the given ID, or `None` if there's no language server attached to
    /// it.
    fn lsp_root(
        id: <Self::Buffer<'_> as Buffer>::Id,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<Option<AbsPathBuf>, Self::LspRootError>;

    /// Prompts the user to select one of the given `(project_root,
    /// session_id)` pairs.
    fn select_session<'pairs>(
        sessions: &'pairs [(AbsPathBuf, Self::SessionId)],
        action: ActionForSelectedSession,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Option<&'pairs (AbsPathBuf, Self::SessionId)>>;

    /// TODO: docs.
    fn start_session(
        args: StartArgs<'_>,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Result<SessionInfos<Self>, Self::StartSessionError>>;
}

/// TODO: docs
pub enum ActionForSelectedSession {
    /// TODO: docs
    CopySessionId,

    /// TODO: docs
    Leave,
}

/// TODO: docs.
#[allow(dead_code)]
pub struct StartArgs<'a> {
    /// TODO: docs.
    pub auth_infos: &'a auth::AuthInfos,

    /// TODO: docs.
    pub project_name: &'a fs::FsNodeName,

    /// TODO: docs.
    pub server_address: &'a config::ServerAddress,
}

/// TODO: docs.
pub struct JoinArgs<'a, B: CollabBackend> {
    /// TODO: docs.
    pub auth_infos: &'a auth::AuthInfos,

    /// TODO: docs.
    pub session_id: B::SessionId,

    /// TODO: docs.
    pub server_address: &'a config::ServerAddress,
}

/// TODO: docs.
pub struct SessionInfos<B: CollabBackend> {
    /// TODO: docs.
    pub host_id: PeerId,

    /// TODO: docs.
    pub local_peer: Peer,

    /// TODO: docs.
    pub project_name: FsNodeNameBuf,

    /// TODO: docs.
    pub remote_peers: Peers,

    /// TODO: docs.
    pub server_tx: B::ServerTx,

    /// TODO: docs.
    pub server_rx: B::ServerRx,

    /// TODO: docs.
    pub session_id: B::SessionId,
}
