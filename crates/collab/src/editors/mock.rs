#![allow(missing_docs)]

use core::convert::Infallible;
use core::error::Error;
use core::ops::Range;
use core::{fmt, ops};

use abs_path::{AbsPath, AbsPathBuf};
pub use collab_server::test::TestSessionId as MockSessionId;
use collab_types::{Peer, PeerHandle};
use duplex_stream::{DuplexStream, duplex};
use editor::context::Borrowed;
use editor::{AgentId, ByteOffset, Context, Editor, EditorAdapter};

use crate::editors::{ActionForSelectedSession, CollabEditor};
use crate::project::Project;
use crate::session::{SessionError, SessionInfos};
use crate::{config, copy_id, jump, leave, pause, resume};

#[allow(clippy::type_complexity)]
pub struct CollabMock<Ed: Editor, F = ()> {
    inner: Ed,
    confirm_start_with: Option<Box<dyn FnMut(&AbsPath) -> bool>>,
    clipboard: Option<MockSessionId>,
    default_dir_for_remote_projects: Option<AbsPathBuf>,
    lsp_root_with: Option<Box<dyn FnMut(Ed::BufferId) -> Option<AbsPathBuf>>>,
    project_filter_with: Box<dyn FnMut(&<Ed::Fs as fs::Fs>::Directory) -> F>,
    select_session_with: Option<
        Box<
            dyn FnMut(
                &[(AbsPathBuf, MockSessionId)],
                ActionForSelectedSession,
            ) -> Option<&(AbsPathBuf, MockSessionId)>,
        >,
    >,
    server_tx: Option<flume::Sender<DuplexStream>>,
}

pub struct CollabServer {
    inner: collab_server::CollabServer<MockConfig>,
    conn_rx: flume::Receiver<DuplexStream>,
    conn_tx: flume::Sender<DuplexStream>,
}

#[derive(Default)]
pub struct MockConfig {
    inner: collab_server::test::TestConfig,
}

#[derive(Default)]
pub struct MockAuthenticator;

pub struct MockParams;

#[derive(Debug)]
pub struct AnyError {
    inner: Box<dyn Error>,
}

#[derive(Debug, derive_more::Display, cauchy::Error)]
#[display("no default directory for remote projects configured")]
pub struct NoDefaultDirForRemoteProjectsError;

impl<Ed: Editor> CollabMock<Ed, ()> {
    pub fn new(inner: Ed) -> Self {
        Self {
            clipboard: None,
            confirm_start_with: None,
            default_dir_for_remote_projects: None,
            inner,
            lsp_root_with: None,
            project_filter_with: Box::new(|_| ()),
            select_session_with: None,
            server_tx: None,
        }
    }
}

impl<Ed, F> CollabMock<Ed, F>
where
    Ed: Editor,
    F: fs::filter::Filter<Ed::Fs, Error: Send> + Send + Sync + 'static,
{
    pub fn confirm_start_with(
        mut self,
        fun: impl FnMut(&AbsPath) -> bool + 'static,
    ) -> Self {
        self.confirm_start_with = Some(Box::new(fun) as _);
        self
    }

    pub fn lsp_root_with(
        mut self,
        fun: impl FnMut(Ed::BufferId) -> Option<AbsPathBuf> + 'static,
    ) -> Self {
        self.lsp_root_with = Some(Box::new(fun) as _);
        self
    }

    pub fn select_session_with(
        mut self,
        fun: impl FnMut(
            &[(AbsPathBuf, MockSessionId)],
            ActionForSelectedSession,
        ) -> Option<&(AbsPathBuf, MockSessionId)>
        + 'static,
    ) -> Self {
        self.select_session_with = Some(Box::new(fun) as _);
        self
    }

    pub fn with_default_dir_for_remote_projects(
        mut self,
        dir_path: impl AsRef<AbsPath>,
    ) -> Self {
        self.default_dir_for_remote_projects =
            Some(dir_path.as_ref().to_owned());
        self
    }

    pub fn with_project_filter<Fun, NewF>(
        self,
        project_filter: Fun,
    ) -> CollabMock<Ed, NewF>
    where
        Fun: FnMut(&<Ed::Fs as fs::Fs>::Directory) -> NewF + 'static,
        NewF: fs::filter::Filter<Ed::Fs, Error: Send> + Send + Sync + 'static,
    {
        CollabMock {
            inner: self.inner,
            confirm_start_with: self.confirm_start_with,
            clipboard: self.clipboard,
            default_dir_for_remote_projects: self
                .default_dir_for_remote_projects,
            lsp_root_with: self.lsp_root_with,
            project_filter_with: Box::new(project_filter),
            select_session_with: self.select_session_with,
            server_tx: self.server_tx,
        }
    }

    pub fn with_server(mut self, server: &CollabServer) -> Self {
        self.server_tx = Some(server.conn_tx.clone());
        self
    }
}

impl CollabServer {
    pub async fn run(self) {
        self.inner.run(self.conn_rx.into_stream()).await;
    }
}

impl AnyError {
    pub fn downcast_ref<T: Error + 'static>(&self) -> Option<&T> {
        self.inner.downcast_ref()
    }

    fn from_str(s: &str) -> Self {
        struct StrError(String);

        impl fmt::Debug for StrError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self.0, f)
            }
        }

        impl fmt::Display for StrError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl Error for StrError {}

        Self::new(StrError(s.to_owned()))
    }

    fn new<E: Error + 'static>(err: E) -> Self {
        Self { inner: Box::new(err) as _ }
    }
}

impl<Ed, F> CollabEditor for CollabMock<Ed, F>
where
    Ed: Editor,
    F: fs::filter::Filter<Ed::Fs, Error: Send> + Send + Sync + 'static,
{
    type Io = DuplexStream;
    type PeerSelection = ();
    type PeerTooltip = ByteOffset;
    type ProgressReporter = ();
    type ProjectFilter = F;
    type ServerParams = MockParams;

    type ConnectToServerError = AnyError;
    type DefaultDirForRemoteProjectsError = NoDefaultDirForRemoteProjectsError;
    type LspRootError = Infallible;
    type ProjectFilterError = Infallible;

    async fn confirm_start(
        project_root: &AbsPath,
        ctx: &mut Context<Self>,
    ) -> bool {
        ctx.with_editor(|this| match &mut this.confirm_start_with {
            Some(fun) => fun(project_root),
            None => true,
        })
    }

    async fn connect_to_server(
        _: config::ServerAddress<'static>,
        ctx: &mut Context<Self>,
    ) -> Result<Self::Io, Self::ConnectToServerError> {
        let server_tx = ctx
            .with_editor(|this| this.server_tx.clone())
            .ok_or(AnyError::from_str("no server set"))?;

        let (client_io, server_io) = duplex(usize::MAX);

        server_tx.send(server_io)?;

        Ok(client_io)
    }

    fn create_peer_selection(
        _remote_peer: Peer,
        _selected_range: Range<ByteOffset>,
        _buffer_id: Self::BufferId,
        _ctx: &mut Context<Self>,
    ) -> Self::PeerSelection {
    }

    fn create_peer_tooltip(
        _remote_peer: Peer,
        tooltip_offset: ByteOffset,
        _buffer_id: Self::BufferId,
        _ctx: &mut Context<Self>,
    ) -> Self::PeerTooltip {
        tooltip_offset
    }

    async fn default_dir_for_remote_projects(
        ctx: &mut Context<Self>,
    ) -> Result<AbsPathBuf, Self::DefaultDirForRemoteProjectsError> {
        ctx.with_editor(|this| {
            this.default_dir_for_remote_projects
                .clone()
                .ok_or(NoDefaultDirForRemoteProjectsError)
        })
    }

    async fn jump_to(
        _buffer_id: Self::BufferId,
        _offset: ByteOffset,
        _agent_id: AgentId,
        _ctx: &mut Context<Self>,
    ) {
        todo!();
    }

    fn lsp_root(
        buffer_id: Self::BufferId,
        ctx: &mut Context<Self>,
    ) -> Result<Option<AbsPathBuf>, Self::LspRootError> {
        Ok(ctx.with_editor(|this| this.lsp_root_with.as_mut()?(buffer_id)))
    }

    fn move_peer_selection(
        _selection: &mut Self::PeerSelection,
        _selected_range: Range<ByteOffset>,
        _ctx: &mut Context<Self>,
    ) {
    }

    fn move_peer_tooltip(
        tooltip: &mut Self::PeerTooltip,
        tooltip_offset: ByteOffset,
        _ctx: &mut Context<Self>,
    ) {
        *tooltip = tooltip_offset;
    }

    fn on_copied_session_id(_: MockSessionId, _: &mut Context<Self>) {}

    fn on_copy_session_id_error(
        _: copy_id::CopyIdError<Self>,
        _: &mut Context<Self>,
    ) {
    }

    fn on_init(_: &mut Context<Self, Borrowed>) {}

    fn on_jump_error(_: jump::JumpError<Self>, _: &mut Context<Self>) {}

    fn on_leave_error(_: leave::LeaveError, _: &mut Context<Self>) {}

    fn on_pause_error(_: pause::PauseError<Self>, _: &mut Context<Self>) {}

    fn on_peer_left(_: &Peer, _: &Project<Self>, _: &mut Context<Self>) {}

    fn on_peer_joined(_: &Peer, _: &Project<Self>, _: &mut Context<Self>) {}

    fn on_resume_error(_: resume::ResumeError<Self>, _: &mut Context<Self>) {}

    fn on_session_ended(_: &SessionInfos<Self>, _: &mut Context<Self>) {}

    fn on_session_error(_: SessionError<Self>, _: &mut Context<Self>) {}

    async fn on_session_joined(_: &SessionInfos<Self>, _: &mut Context<Self>) {
    }

    fn on_session_left(_: &SessionInfos<Self>, _: &mut Context<Self>) {}

    async fn on_session_started(
        _: &SessionInfos<Self>,
        _: &mut Context<Self>,
    ) {
    }

    fn project_filter(
        project_root: &<Self::Fs as fs::Fs>::Directory,
        ctx: &mut Context<Self>,
    ) -> Result<Self::ProjectFilter, Self::ProjectFilterError> {
        Ok(ctx.with_editor(|this| {
            this.project_filter_with.as_mut()(project_root)
        }))
    }

    fn remove_peer_selection(
        _selection: Self::PeerSelection,
        _ctx: &mut Context<Self>,
    ) {
    }

    fn remove_peer_tooltip(
        _tooltip: Self::PeerTooltip,
        _ctx: &mut Context<Self>,
    ) {
    }

    async fn select_session<'pairs>(
        sessions: &'pairs [(AbsPathBuf, MockSessionId)],
        action: ActionForSelectedSession,
        ctx: &mut Context<Self>,
    ) -> Option<&'pairs (AbsPathBuf, MockSessionId)> {
        ctx.with_editor(|this| {
            this.select_session_with.as_mut()?(sessions, action)
        })
    }

    fn should_remote_save_cause_local_save(_: &Self::Buffer<'_>) -> bool {
        true
    }
}

impl<Ed: Editor + Default> Default for CollabMock<Ed, ()> {
    fn default() -> Self {
        Self::new(Ed::default())
    }
}

impl<Ed: Editor, F> ops::Deref for CollabMock<Ed, F> {
    type Target = Ed;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<Ed: Editor, F> ops::DerefMut for CollabMock<Ed, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<Ed: Editor, F: 'static> EditorAdapter for CollabMock<Ed, F> {}

impl Default for CollabServer {
    fn default() -> Self {
        let (conn_tx, conn_rx) = flume::unbounded();
        Self { conn_rx, conn_tx, inner: Default::default() }
    }
}

#[derive(
    Debug,
    derive_more::Display,
    PartialEq,
    Eq,
    cauchy::Error,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum Never {}

impl collab_server::Config for MockConfig {
    type Authenticator = MockAuthenticator;
    type Executor =
        <collab_server::test::TestConfig as collab_server::Config>::Executor;
    type Params = MockParams;

    fn authenticator(&self) -> &Self::Authenticator {
        &MockAuthenticator
    }

    fn executor(&self) -> &Self::Executor {
        self.inner.executor()
    }

    fn new_session_id(&self) -> MockSessionId {
        self.inner.new_session_id()
    }
}

impl collab_server::Authenticator for MockAuthenticator {
    type Infos = collab_types::PeerHandle;
    type Error = Never;

    async fn authenticate(
        &self,
        peer_handle: &Self::Infos,
    ) -> Result<PeerHandle, Self::Error> {
        Ok(peer_handle.clone())
    }
}

impl collab_server::Params for MockParams {
    const MAX_FRAME_LEN: u32 = Self::MIN_FRAME_LEN;

    type AuthenticateInfos = collab_types::PeerHandle;
    type AuthenticateError = Never;
    type SessionId = MockSessionId;
}

impl<E: Error + 'static> From<E> for AnyError {
    fn from(err: E) -> Self {
        Self::new(err)
    }
}

impl PartialEq for AnyError {
    fn eq(&self, other: &Self) -> bool {
        self.inner.to_string() == other.inner.to_string()
    }
}
