#[cfg(feature = "mock")]
pub mod mock;
#[cfg(feature = "neovim")]
mod neovim;

use core::fmt::Debug;

use collab_server::Authenticator;
use ed::backend::Backend;
use ed::command::CommandArgs;
use ed::fs::{self, AbsPath, AbsPathBuf};
use ed::{Context, notify};
use futures_util::{AsyncRead, AsyncWrite};

use crate::config;

/// A [`Backend`] subtrait defining additional capabilities needed by the
/// actions in this crate.
pub trait CollabBackend: Backend {
    /// TODO: docs.
    type Io: AsyncRead + AsyncWrite + Unpin;

    /// TODO: docs.
    type ProjectFilter: walkdir::Filter<Self::Fs, Error: Send> + Send + Sync;

    /// TODO: docs.
    type ServerConfig: collab_server::Config<
            Authenticator: Authenticator<Infos: From<auth::AuthInfos>>,
            SessionId: for<'a> TryFrom<CommandArgs<'a>, Error: notify::Error>,
        >;

    /// The type of error returned by
    /// [`connect_to_server`](CollabBackend::connect_to_server).
    type ConnectToServerError: Debug + notify::Error;

    /// The type of error returned by
    /// [`copy_session_id`](CollabBackend::copy_session_id).
    type CopySessionIdError: Debug + notify::Error;

    /// The type of error returned by
    /// [`default_dir_for_remote_projects`](CollabBackend::default_dir_for_remote_projects).
    type DefaultDirForRemoteProjectsError: Debug + notify::Error;

    /// The type of error returned by [`home_dir`](CollabBackend::home_dir).
    type HomeDirError: Debug + notify::Error;

    /// The type of error returned by [`lsp_root`](CollabBackend::lsp_root).
    type LspRootError: Debug + notify::Error;

    /// Asks the user to confirm starting a new collaborative editing session
    /// rooted at the given path.
    fn confirm_start(
        project_root: &AbsPath,
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = bool>;

    /// TODO: docs.
    fn connect_to_server(
        server_addr: config::ServerAddress,
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = Result<Self::Io, Self::ConnectToServerError>>;

    /// Copies the given [`SessionId`] to the user's clipboard.
    fn copy_session_id(
        session_id: SessionId<Self>,
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = Result<(), Self::CopySessionIdError>>;

    /// TODO: docs.
    fn default_dir_for_remote_projects(
        ctx: &mut Context<Self>,
    ) -> impl Future<
        Output = Result<AbsPathBuf, Self::DefaultDirForRemoteProjectsError>,
    >;

    /// Returns the absolute path to the user's home directory.
    fn home_dir(
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = Result<AbsPathBuf, Self::HomeDirError>>;

    /// Returns the path to the root of the workspace containing the buffer
    /// with the given ID, or `None` if there's no language server attached to
    /// it.
    fn lsp_root(
        id: Self::BufferId,
        ctx: &mut Context<Self>,
    ) -> Result<Option<AbsPathBuf>, Self::LspRootError>;

    /// TODO: docs.
    fn project_filter(
        project_root: &<Self::Fs as fs::Fs>::Directory,
        ctx: &mut Context<Self>,
    ) -> Self::ProjectFilter;

    /// Prompts the user to select one of the given `(project_root,
    /// session_id)` pairs.
    fn select_session<'pairs>(
        sessions: &'pairs [(AbsPathBuf, SessionId<Self>)],
        action: ActionForSelectedSession,
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = Option<&'pairs (AbsPathBuf, SessionId<Self>)>>;
}

/// TODO: docs
pub enum ActionForSelectedSession {
    /// TODO: docs
    CopySessionId,

    /// TODO: docs
    Leave,
}

/// TODO: docs.
pub(crate) type MessageRx<B> = collab_server::client::ClientRx<Reader<B>>;

/// TODO: docs.
pub(crate) type MessageTx<B> = collab_server::client::ClientTx<Writer<B>>;

/// TODO: docs.
pub(crate) type Reader<B> =
    futures_util::io::ReadHalf<<B as CollabBackend>::Io>;

/// TODO: docs.
pub(crate) type Writer<B> =
    futures_util::io::WriteHalf<<B as CollabBackend>::Io>;

/// TODO: docs.
pub(crate) type SessionId<B> =
    <<B as CollabBackend>::ServerConfig as collab_server::Config>::SessionId;

/// TODO: docs.
pub(crate) type Welcome<B> =
    collab_server::client::Welcome<Reader<B>, Writer<B>, SessionId<B>>;
