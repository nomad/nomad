#[cfg(feature = "mock")]
pub mod mock;
#[cfg(feature = "neovim")]
mod neovim;

use core::error::Error;
use core::fmt::Debug;
use core::ops::Range;
use core::str::FromStr;

use abs_path::{AbsPath, AbsPathBuf};
use collab_types::Peer;
use editor::{ByteOffset, Context, Editor};
use futures_util::{AsyncRead, AsyncWrite};

use crate::session::SessionInfos;
use crate::{config, join, leave, session, start, yank};

/// An [`Editor`] subtrait defining additional capabilities needed by the
/// actions in this crate.
pub trait CollabEditor: Editor {
    /// TODO: docs.
    type Io: AsyncRead + AsyncWrite + Unpin;

    /// The type representing a text selection created by a remote peer in a
    /// given buffer.
    type PeerSelection;

    /// TODO: docs.
    type PeerTooltip;

    /// TODO: docs.
    type ProjectFilter: fs::filter::Filter<Self::Fs, Error: Send> + Send + Sync;

    /// TODO: docs.
    type ServerParams: collab_types::Params<
            AuthenticateInfos: From<auth::AuthInfos>,
            SessionId: FromStr<Err: Error>,
        >;

    /// The type of error returned by
    /// [`connect_to_server`](CollabEditor::connect_to_server).
    type ConnectToServerError: Debug;

    /// The type of error returned by
    /// [`copy_session_id`](CollabEditor::copy_session_id).
    type CopySessionIdError: Debug;

    /// The type of error returned by
    /// [`default_dir_for_remote_projects`](CollabEditor::default_dir_for_remote_projects).
    type DefaultDirForRemoteProjectsError: Debug;

    /// The type of error returned by [`home_dir`](CollabEditor::home_dir).
    type HomeDirError: Debug;

    /// The type of error returned by [`lsp_root`](CollabEditor::lsp_root).
    type LspRootError: Debug;

    /// The type of error returned by
    /// [`project_filter`](CollabEditor::project_filter).
    type ProjectFilterError: Error + Send;

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
    fn create_peer_selection(
        remote_peer: Peer,
        selected_range: Range<ByteOffset>,
        buffer_id: Self::BufferId,
        ctx: &mut Context<Self>,
    ) -> Self::PeerSelection;

    /// TODO: docs.
    fn create_peer_tooltip(
        remote_peer: Peer,
        tooltip_offset: ByteOffset,
        buffer_id: Self::BufferId,
        ctx: &mut Context<Self>,
    ) -> Self::PeerTooltip;

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
    fn move_peer_selection(
        selection: &mut Self::PeerSelection,
        offset_range: Range<ByteOffset>,
        ctx: &mut Context<Self>,
    );

    /// TODO: docs.
    fn move_peer_tooltip(
        tooltip: &mut Self::PeerTooltip,
        tooltip_offset: ByteOffset,
        ctx: &mut Context<Self>,
    );

    /// Called when the [`Join`](join::Join) action returns an error.
    fn on_join_error(error: join::JoinError<Self>, ctx: &mut Context<Self>);

    /// Called when the [`Leave`](leave::Leave) action returns an error.
    fn on_leave_error(error: leave::LeaveError, ctx: &mut Context<Self>);

    /// Called when running a session returns an error.
    fn on_session_error(
        error: session::SessionError<Self>,
        ctx: &mut Context<Self>,
    );

    /// Called after the [`Start`](start::Start) action successfully starts a
    /// new session, just before running the session's event loop.
    fn on_session_started(
        session_infos: &SessionInfos<Self>,
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = ()>;

    /// Called when the [`Start`](start::Start) action returns an error.
    fn on_start_error(error: start::StartError<Self>, ctx: &mut Context<Self>);

    /// Called when the [`Yank`](yank::Yank) action returns an error.
    fn on_yank_error(error: yank::YankError<Self>, ctx: &mut Context<Self>);

    /// TODO: docs.
    fn project_filter(
        project_root: &<Self::Fs as fs::Fs>::Directory,
        ctx: &mut Context<Self>,
    ) -> Result<Self::ProjectFilter, Self::ProjectFilterError>;

    /// TODO: docs.
    fn remove_peer_selection(
        selection: Self::PeerSelection,
        ctx: &mut Context<Self>,
    );

    /// TODO: docs.
    fn remove_peer_tooltip(
        tooltip: Self::PeerTooltip,
        ctx: &mut Context<Self>,
    );

    /// Prompts the user to select one of the given `(project_root,
    /// session_id)` pairs.
    fn select_session<'pairs>(
        sessions: &'pairs [(AbsPathBuf, SessionId<Self>)],
        action: ActionForSelectedSession,
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = Option<&'pairs (AbsPathBuf, SessionId<Self>)>>;

    /// TODO: docs.
    fn should_remote_save_cause_local_save(buffer: &Self::Buffer<'_>) -> bool;
}

/// TODO: docs
pub enum ActionForSelectedSession {
    /// TODO: docs
    CopySessionId,

    /// TODO: docs
    Leave,
}

/// TODO: docs.
pub type SessionId<Ed> =
    <<Ed as CollabEditor>::ServerParams as collab_types::Params>::SessionId;

/// TODO: docs.
pub(crate) type MessageRx<Ed> = collab_server::client::Receiver<Reader<Ed>>;

/// TODO: docs.
pub(crate) type MessageTx<Ed> = collab_server::client::Sender<Writer<Ed>>;

/// TODO: docs.
pub(crate) type Reader<Ed> =
    futures_util::io::ReadHalf<<Ed as CollabEditor>::Io>;

/// TODO: docs.
pub(crate) type Writer<Ed> =
    futures_util::io::WriteHalf<<Ed as CollabEditor>::Io>;

/// TODO: docs.
pub(crate) type Welcome<Ed> =
    collab_server::client::Welcome<Reader<Ed>, Writer<Ed>, SessionId<Ed>>;
