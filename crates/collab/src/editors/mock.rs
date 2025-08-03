#![allow(missing_docs)]

use core::convert::Infallible;
use core::error::Error;
use core::fmt;
use core::num::NonZeroU32;
use core::ops::Range;
use core::str::FromStr;

use abs_path::{AbsPath, AbsPathBuf};
use collab_server::Config;
use collab_server::test::{TestConfig as InnerConfig, TestSessionId};
use collab_types::{Peer, PeerId};
use duplex_stream::{DuplexStream, duplex};
use ed::notify::{self, MaybeResult};
use ed::{
    AgentId,
    ApiValue,
    BaseEditor,
    BorrowState,
    ByteOffset,
    Context,
    Editor,
    fs,
};
use serde::{Deserialize, Serialize};

use crate::config;
use crate::editors::{ActionForSelectedSession, CollabEditor};

#[allow(clippy::type_complexity)]
pub struct CollabMock<Ed: Editor, F = ()> {
    inner: Ed,
    confirm_start_with: Option<Box<dyn FnMut(&AbsPath) -> bool>>,
    clipboard: Option<SessionId>,
    default_dir_for_remote_projects: Option<AbsPathBuf>,
    home_dir: Option<AbsPathBuf>,
    lsp_root_with: Option<Box<dyn FnMut(Ed::BufferId) -> Option<AbsPathBuf>>>,
    project_filter_with: Box<dyn FnMut(&<Ed::Fs as fs::Fs>::Directory) -> F>,
    select_session_with: Option<
        Box<
            dyn FnMut(
                &[(AbsPathBuf, SessionId)],
                ActionForSelectedSession,
            ) -> Option<&(AbsPathBuf, SessionId)>,
        >,
    >,
    server_tx: Option<flume::Sender<DuplexStream>>,
}

pub struct CollabServer {
    inner: collab_server::CollabServer<ServerConfig>,
    conn_rx: flume::Receiver<DuplexStream>,
    conn_tx: flume::Sender<DuplexStream>,
}

#[derive(Default)]
pub struct Authenticator;

#[derive(Default)]
pub struct ServerConfig {
    inner: InnerConfig,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
pub struct SessionId(pub u64);

#[derive(Debug)]
pub struct AnyError {
    inner: Box<dyn Error>,
}

#[derive(Debug)]
pub struct NoDefaultDirForRemoteProjectsError;

impl<Ed: Editor> CollabMock<Ed, ()> {
    pub fn new(inner: Ed) -> Self {
        Self {
            clipboard: None,
            confirm_start_with: None,
            default_dir_for_remote_projects: None,
            home_dir: None,
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
    F: walkdir::Filter<Ed::Fs, Error: Send> + Send + Sync + 'static,
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
            &[(AbsPathBuf, SessionId)],
            ActionForSelectedSession,
        ) -> Option<&(AbsPathBuf, SessionId)>
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

    pub fn with_home_dir(mut self, dir_path: AbsPathBuf) -> Self {
        self.home_dir = Some(dir_path);
        self
    }

    pub fn with_project_filter<Fun, NewF>(
        self,
        project_filter: Fun,
    ) -> CollabMock<Ed, NewF>
    where
        Fun: FnMut(&<Ed::Fs as fs::Fs>::Directory) -> NewF + 'static,
        NewF: walkdir::Filter<Ed::Fs, Error: Send> + Send + Sync + 'static,
    {
        CollabMock {
            inner: self.inner,
            confirm_start_with: self.confirm_start_with,
            clipboard: self.clipboard,
            default_dir_for_remote_projects: self
                .default_dir_for_remote_projects,
            home_dir: self.home_dir,
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
        let (done_tx, done_rx) = flume::bounded::<()>(1);

        std::thread::spawn(move || {
            self.inner.run(self.conn_rx.into_stream());
            let _ = done_tx.send(());
        });

        done_rx.recv_async().await.expect("tx is still alive");
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
    Ed: BaseEditor,
    F: walkdir::Filter<Ed::Fs, Error: Send> + Send + Sync + 'static,
{
    type Io = DuplexStream;
    type PeerSelection = ();
    type PeerTooltip = ();
    type ProjectFilter = F;
    type ServerConfig = ServerConfig;

    type ConnectToServerError = AnyError;
    type CopySessionIdError = Infallible;
    type DefaultDirForRemoteProjectsError = NoDefaultDirForRemoteProjectsError;
    type HomeDirError = AnyError;
    type LspRootError = Infallible;

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
        _: config::ServerAddress,
        ctx: &mut Context<Self>,
    ) -> Result<Self::Io, Self::ConnectToServerError> {
        let server_tx = ctx
            .with_editor(|this| this.server_tx.clone())
            .ok_or(AnyError::from_str("no server set"))?;

        let (client_io, server_io) = duplex(usize::MAX);

        server_tx.send(server_io)?;

        Ok(client_io)
    }

    async fn copy_session_id(
        session_id: SessionId,
        ctx: &mut Context<Self>,
    ) -> Result<(), Self::CopySessionIdError> {
        ctx.with_editor(|this| this.clipboard = Some(session_id));
        Ok(())
    }

    async fn create_peer_selection(
        _remote_peer: Peer,
        _selected_range: Range<ByteOffset>,
        _buffer_id: Self::BufferId,
        _ctx: &mut Context<Self>,
    ) -> Self::PeerTooltip {
    }

    async fn create_peer_tooltip(
        _remote_peer: Peer,
        _tooltip_offset: ByteOffset,
        _buffer_id: Self::BufferId,
        _ctx: &mut Context<Self>,
    ) -> Self::PeerTooltip {
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

    async fn home_dir(
        ctx: &mut Context<Self>,
    ) -> Result<AbsPathBuf, Self::HomeDirError> {
        ctx.with_editor(|this| match &this.home_dir {
            Some(path) => Ok(path.clone()),
            None => Err(AnyError::from_str("no home directory configured")),
        })
    }

    fn lsp_root(
        buffer_id: Self::BufferId,
        ctx: &mut Context<Self>,
    ) -> Result<Option<AbsPathBuf>, Self::LspRootError> {
        Ok(ctx.with_editor(|this| this.lsp_root_with.as_mut()?(buffer_id)))
    }

    fn move_peer_selection<'ctx>(
        _selection: &mut Self::PeerSelection,
        _selected_range: Range<ByteOffset>,
        _ctx: &'ctx mut Context<Self>,
    ) -> impl Future<Output = ()> + use<'ctx, Ed, F> {
        async move {}
    }

    fn move_peer_tooltip<'ctx>(
        _tooltip: &mut Self::PeerTooltip,
        _tooltip_offset: ByteOffset,
        _ctx: &'ctx mut Context<Self>,
    ) -> impl Future<Output = ()> + use<'ctx, Ed, F> {
        async move {}
    }

    fn project_filter(
        project_root: &<Self::Fs as fs::Fs>::Directory,
        ctx: &mut Context<Self>,
    ) -> Self::ProjectFilter {
        ctx.with_editor(|this| this.project_filter_with.as_mut()(project_root))
    }

    async fn remove_peer_selection(
        _selection: Self::PeerSelection,
        _ctx: &mut Context<Self>,
    ) -> () {
    }

    async fn remove_peer_tooltip(
        _tooltip: Self::PeerTooltip,
        _ctx: &mut Context<Self>,
    ) {
    }

    async fn select_session<'pairs>(
        sessions: &'pairs [(AbsPathBuf, SessionId)],
        action: ActionForSelectedSession,
        ctx: &mut Context<Self>,
    ) -> Option<&'pairs (AbsPathBuf, SessionId)> {
        ctx.with_editor(|this| {
            this.select_session_with.as_mut()?(sessions, action)
        })
    }

    fn should_remote_save_cause_local_save(_: &Self::Buffer<'_>) -> bool {
        true
    }
}

impl<Ed, F> Editor for CollabMock<Ed, F>
where
    Ed: BaseEditor,
    F: walkdir::Filter<Ed::Fs, Error: Send> + Send + Sync + 'static,
{
    type Api = <Ed as Editor>::Api;
    type Buffer<'a> = <Ed as Editor>::Buffer<'a>;
    type BufferId = <Ed as Editor>::BufferId;
    type Cursor<'a> = <Ed as Editor>::Cursor<'a>;
    type CursorId = <Ed as Editor>::CursorId;
    type Fs = <Ed as Editor>::Fs;
    type Emitter<'this> = <Ed as Editor>::Emitter<'this>;
    type Executor = <Ed as Editor>::Executor;
    type EventHandle = <Ed as Editor>::EventHandle;
    type Selection<'a> = <Ed as Editor>::Selection<'a>;
    type SelectionId = <Ed as Editor>::SelectionId;

    type BufferSaveError = <Ed as Editor>::BufferSaveError;
    type CreateBufferError = <Ed as Editor>::CreateBufferError;
    type SerializeError = <Ed as Editor>::SerializeError;
    type DeserializeError = <Ed as Editor>::DeserializeError;

    fn buffer(&mut self, id: Self::BufferId) -> Option<Self::Buffer<'_>> {
        self.inner.buffer(id)
    }
    fn buffer_at_path(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        self.inner.buffer_at_path(path)
    }
    fn buffer_ids(
        &mut self,
    ) -> impl Iterator<Item = Self::BufferId> + use<Ed, F> {
        self.inner.buffer_ids()
    }
    async fn create_buffer(
        file_path: &AbsPath,
        agent_id: AgentId,
        ctx: &mut Context<Self, impl BorrowState>,
    ) -> Result<Self::BufferId, Self::CreateBufferError> {
        <Ed as BaseEditor>::create_buffer(file_path, agent_id, ctx).await
    }
    fn current_buffer(&mut self) -> Option<Self::Buffer<'_>> {
        self.inner.current_buffer()
    }
    fn cursor(&mut self, id: Self::CursorId) -> Option<Self::Cursor<'_>> {
        self.inner.cursor(id)
    }
    fn fs(&mut self) -> Self::Fs {
        self.inner.fs()
    }
    fn emitter(&mut self) -> Self::Emitter<'_> {
        self.inner.emitter()
    }
    fn executor(&mut self) -> &mut Self::Executor {
        self.inner.executor()
    }
    fn on_buffer_created<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Buffer<'_>, AgentId) + 'static,
    {
        self.inner.on_buffer_created(fun)
    }
    fn on_cursor_created<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Cursor<'_>, AgentId) + 'static,
    {
        self.inner.on_cursor_created(fun)
    }
    fn on_selection_created<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Selection<'_>, AgentId) + 'static,
    {
        self.inner.on_selection_created(fun)
    }
    fn reinstate_panic_hook(&self) -> bool {
        self.inner.reinstate_panic_hook()
    }
    fn selection(
        &mut self,
        id: Self::SelectionId,
    ) -> Option<Self::Selection<'_>> {
        self.inner.selection(id)
    }
    fn serialize<V>(
        &mut self,
        value: &V,
    ) -> impl MaybeResult<ApiValue<Self>, Error = Self::SerializeError>
    where
        V: ?Sized + Serialize,
    {
        self.inner.serialize(value)
    }
    fn deserialize<'de, V>(
        &mut self,
        value: ApiValue<Self>,
    ) -> impl MaybeResult<V, Error = Self::DeserializeError>
    where
        V: Deserialize<'de>,
    {
        self.inner.deserialize(value)
    }
}

impl<Ed: Editor, F> AsMut<Ed> for CollabMock<Ed, F> {
    fn as_mut(&mut self) -> &mut Ed {
        &mut self.inner
    }
}

impl Config for ServerConfig {
    const MAX_FRAME_LEN: NonZeroU32 = <InnerConfig as Config>::MAX_FRAME_LEN;
    const SERVER_PEER_ID: PeerId = <InnerConfig as Config>::SERVER_PEER_ID;

    type Authenticator = Authenticator;
    type Executor = <InnerConfig as Config>::Executor;
    type SessionId = SessionId;

    fn authenticator(&self) -> &Self::Authenticator {
        &Authenticator
    }
    fn executor(&self) -> &Self::Executor {
        self.inner.executor()
    }
    fn new_session_id(&self) -> Self::SessionId {
        self.inner.new_session_id().into()
    }
}

impl Default for CollabServer {
    fn default() -> Self {
        let (conn_tx, conn_rx) = flume::unbounded();
        Self { conn_rx, conn_tx, inner: Default::default() }
    }
}

impl<Ed: Editor + Default> Default for CollabMock<Ed, ()> {
    fn default() -> Self {
        Self::new(Ed::default())
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

impl collab_server::Authenticator for Authenticator {
    type Infos = auth::AuthInfos;
    type Error = Never;

    async fn authenticate(&self, _: &Self::Infos) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl FromStr for SessionId {
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(SessionId)
    }
}

impl<'a> TryFrom<ed::command::CommandArgs<'a>> for SessionId {
    type Error = <ed::command::Parse<Self> as TryFrom<
        ed::command::CommandArgs<'a>,
    >>::Error;

    fn try_from(
        args: ed::command::CommandArgs<'a>,
    ) -> Result<Self, Self::Error> {
        ed::command::Parse::<Self>::try_from(args)
            .map(|ed::command::Parse(this)| this)
    }
}

impl From<TestSessionId> for SessionId {
    fn from(TestSessionId(session_id): TestSessionId) -> Self {
        Self(session_id)
    }
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

impl notify::Error for AnyError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        self.inner.to_message()
    }
}

impl notify::Error for NoDefaultDirForRemoteProjectsError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        (
            notify::Level::Error,
            notify::Message::from_str(
                "no default directory for remote projects configured",
            ),
        )
    }
}
