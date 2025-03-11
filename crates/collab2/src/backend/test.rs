//! TODO: docs.

#![allow(missing_docs)]

use core::convert::Infallible;
use core::error::Error;
use core::fmt;
use core::pin::Pin;
use core::str::FromStr;
use core::task::{Context, Poll};
use std::io;

use collab_server::message::{Message, Peer};
use collab_server::test::{TestConfig, TestSessionId};
use collab_server::{Knock, SessionIntent, client};
use duplex_stream::{DuplexStream, duplex};
use futures_util::io::{ReadHalf, WriteHalf};
use futures_util::{AsyncReadExt, Sink, Stream};
use nvimx2::AsyncCtx;
use nvimx2::backend::{ApiValue, Backend, Buffer, BufferId};
use nvimx2::fs::{AbsPath, AbsPathBuf};
use nvimx2::notify::{self, MaybeResult};
use serde::{Deserialize, Serialize};

use crate::backend::{
    ActionForSelectedSession,
    CollabBackend,
    JoinArgs,
    SessionInfos,
    StartArgs,
};

#[allow(clippy::type_complexity)]
pub struct CollabTestBackend<B: Backend> {
    inner: B,
    confirm_start_with: Option<Box<dyn FnMut(&AbsPath) -> bool>>,
    clipboard: Option<SessionId>,
    default_dir_for_remote_projects: Option<AbsPathBuf>,
    home_dir: Option<AbsPathBuf>,
    lsp_root_with: Option<
        Box<dyn FnMut(<B::Buffer<'_> as Buffer>::Id) -> Option<AbsPathBuf>>,
    >,
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

pub struct CollabTestServer {
    inner: collab_server::CollabServer<TestConfig>,
    conn_rx: flume::Receiver<DuplexStream>,
    conn_tx: flume::Sender<DuplexStream>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(transparent)]
pub struct SessionId(pub u64);

pin_project_lite::pin_project! {
    pub struct TestRx {
        #[pin]
        inner: client::ClientRx<ReadHalf<DuplexStream>>,
    }
}

pin_project_lite::pin_project! {
    pub struct TestTx {
        #[pin]
        inner: client::ClientTx<WriteHalf<DuplexStream>>,
    }
}

#[derive(Debug)]
pub struct AnyError {
    inner: Box<dyn Error>,
}

#[derive(Debug)]
pub struct NoDefaultDirForRemoteProjectsError;

#[derive(Debug)]
pub struct TestTxError {
    inner: io::Error,
}

#[derive(Debug)]
pub struct TestRxError {
    inner: client::ClientRxError,
}

impl<B: Backend> CollabTestBackend<B> {
    pub fn confirm_start_with(
        mut self,
        fun: impl FnMut(&AbsPath) -> bool + 'static,
    ) -> Self {
        self.confirm_start_with = Some(Box::new(fun) as _);
        self
    }

    pub fn lsp_root_with(
        mut self,
        fun: impl FnMut(<B::Buffer<'_> as Buffer>::Id) -> Option<AbsPathBuf>
        + 'static,
    ) -> Self {
        self.lsp_root_with = Some(Box::new(fun) as _);
        self
    }

    pub fn new(inner: B) -> Self {
        Self {
            clipboard: None,
            confirm_start_with: None,
            default_dir_for_remote_projects: None,
            home_dir: None,
            inner,
            lsp_root_with: None,
            select_session_with: None,
            server_tx: None,
        }
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

    pub fn with_server(mut self, server: &CollabTestServer) -> Self {
        self.server_tx = Some(server.conn_tx.clone());
        self
    }
}

impl CollabTestServer {
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

impl<B: Backend> CollabBackend for CollabTestBackend<B> {
    type ServerRx = TestRx;
    type ServerTx = TestTx;
    type SessionId = SessionId;

    type CopySessionIdError = Infallible;
    type DefaultDirForRemoteProjectsError = NoDefaultDirForRemoteProjectsError;
    type HomeDirError = AnyError;
    type JoinSessionError = AnyError;
    type LspRootError = Infallible;
    type ServerRxError = TestRxError;
    type ServerTxError = TestTxError;
    type StartSessionError = AnyError;

    async fn confirm_start(
        project_root: &AbsPath,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> bool {
        ctx.with_backend(|this| match &mut this.confirm_start_with {
            Some(fun) => fun(project_root),
            None => true,
        })
    }

    async fn copy_session_id(
        session_id: Self::SessionId,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<(), Self::CopySessionIdError> {
        ctx.with_backend(|this| this.clipboard = Some(session_id));
        Ok(())
    }

    async fn default_dir_for_remote_projects(
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<AbsPathBuf, Self::DefaultDirForRemoteProjectsError> {
        ctx.with_backend(|this| {
            this.default_dir_for_remote_projects
                .clone()
                .ok_or(NoDefaultDirForRemoteProjectsError)
        })
    }

    async fn home_dir(
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<AbsPathBuf, Self::HomeDirError> {
        ctx.with_backend(|this| match &this.home_dir {
            Some(path) => Ok(path.clone()),
            None => Err(AnyError::from_str("no home directory configured")),
        })
    }

    async fn join_session(
        args: JoinArgs<'_, Self>,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<SessionInfos<Self>, Self::JoinSessionError> {
        let server_tx = ctx
            .with_backend(|this| this.server_tx.clone())
            .ok_or(AnyError::from_str("no server set"))?;

        let (client_io, server_io) = duplex(usize::MAX);

        server_tx.send(server_io)?;

        let (reader, writer) = client_io.split();

        let github_handle = args.auth_infos.handle().clone();

        let knock = Knock::<TestConfig> {
            auth_infos: github_handle.clone(),
            session_intent: SessionIntent::JoinExisting(
                args.session_id.into(),
            ),
        };

        let welcome =
            client::Knocker::new(reader, writer).knock(knock).await?;

        Ok(SessionInfos {
            host_id: welcome.host_id,
            local_peer: Peer::new(welcome.peer_id, github_handle),
            project_name: welcome.project_name,
            remote_peers: welcome.other_peers,
            server_rx: TestRx { inner: welcome.rx },
            server_tx: TestTx { inner: welcome.tx },
            session_id: welcome.session_id.into(),
        })
    }

    fn lsp_root(
        buffer_id: <Self::Buffer<'_> as Buffer>::Id,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<Option<AbsPathBuf>, Self::LspRootError> {
        Ok(ctx.with_backend(|this| this.lsp_root_with.as_mut()?(buffer_id)))
    }

    async fn select_session<'pairs>(
        sessions: &'pairs [(AbsPathBuf, Self::SessionId)],
        action: ActionForSelectedSession,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Option<&'pairs (AbsPathBuf, Self::SessionId)> {
        ctx.with_backend(|this| {
            this.select_session_with.as_mut()?(sessions, action)
        })
    }

    async fn start_session(
        args: StartArgs<'_>,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<SessionInfos<Self>, Self::StartSessionError> {
        let server_tx = ctx
            .with_backend(|this| this.server_tx.clone())
            .ok_or(AnyError::from_str("no server set"))?;

        let (client_io, server_io) = duplex(usize::MAX);

        server_tx.send(server_io)?;

        let (reader, writer) = client_io.split();

        let github_handle = args.auth_infos.handle().clone();

        let knock = Knock::<TestConfig> {
            auth_infos: github_handle.clone(),
            session_intent: SessionIntent::StartNew(
                args.project_name.to_owned(),
            ),
        };

        let welcome = client::Knocker::<_, _, TestConfig>::new(reader, writer)
            .knock(knock)
            .await?;

        Ok(SessionInfos {
            host_id: welcome.host_id,
            local_peer: Peer::new(welcome.peer_id, github_handle),
            project_name: welcome.project_name,
            remote_peers: welcome.other_peers,
            server_rx: TestRx { inner: welcome.rx },
            server_tx: TestTx { inner: welcome.tx },
            session_id: welcome.session_id.into(),
        })
    }
}

impl<B: Backend> Backend for CollabTestBackend<B> {
    const REINSTATE_PANIC_HOOK: bool = B::REINSTATE_PANIC_HOOK;

    type Api = <B as Backend>::Api;
    type Buffer<'a> = <B as Backend>::Buffer<'a>;
    type BufferId = <B as Backend>::BufferId;
    type LocalExecutor = <B as Backend>::LocalExecutor;
    type BackgroundExecutor = <B as Backend>::BackgroundExecutor;
    type Fs = <B as Backend>::Fs;
    type Emitter<'this> = <B as Backend>::Emitter<'this>;
    type SerializeError = <B as Backend>::SerializeError;
    type DeserializeError = <B as Backend>::DeserializeError;

    fn buffer(&mut self, id: BufferId<Self>) -> Option<Self::Buffer<'_>> {
        self.inner.buffer(id)
    }

    fn buffer_ids(&mut self) -> impl Iterator<Item = BufferId<Self>> + use<B> {
        self.inner.buffer_ids()
    }

    fn current_buffer(&mut self) -> Option<Self::Buffer<'_>> {
        self.inner.current_buffer()
    }

    fn fs(&mut self) -> Self::Fs {
        self.inner.fs()
    }

    fn emitter(&mut self) -> Self::Emitter<'_> {
        self.inner.emitter()
    }

    fn local_executor(&mut self) -> &mut Self::LocalExecutor {
        self.inner.local_executor()
    }

    fn focus_buffer_at(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        self.inner.focus_buffer_at(path)
    }

    fn background_executor(&mut self) -> &mut Self::BackgroundExecutor {
        self.inner.background_executor()
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

impl Default for CollabTestServer {
    fn default() -> Self {
        let (conn_tx, conn_rx) = flume::unbounded();
        Self {
            conn_rx,
            conn_tx,
            inner: collab_server::CollabServer::default(),
        }
    }
}

impl<B: Backend + Default> Default for CollabTestBackend<B> {
    fn default() -> Self {
        Self::new(B::default())
    }
}

impl FromStr for SessionId {
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(SessionId)
    }
}

impl From<SessionId> for TestSessionId {
    fn from(SessionId(session_id): SessionId) -> Self {
        Self(session_id)
    }
}

impl From<TestSessionId> for SessionId {
    fn from(TestSessionId(session_id): TestSessionId) -> Self {
        Self(session_id)
    }
}

impl Sink<Message> for TestTx {
    type Error = TestTxError;

    fn poll_ready(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Sink::<Message>::poll_ready(self.project().inner, ctx)
            .map_err(|err| TestTxError { inner: err })
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: Message,
    ) -> Result<(), Self::Error> {
        Sink::<Message>::start_send(self.project().inner, item)
            .map_err(|err| TestTxError { inner: err })
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Sink::<Message>::poll_flush(self.project().inner, ctx)
            .map_err(|err| TestTxError { inner: err })
    }

    fn poll_close(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Sink::<Message>::poll_close(self.project().inner, ctx)
            .map_err(|err| TestTxError { inner: err })
    }
}

impl Stream for TestRx {
    type Item = Result<Message, TestRxError>;

    fn poll_next(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.project()
            .inner
            .poll_next(ctx)
            .map_err(|err| TestRxError { inner: err })
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

impl notify::Error for TestRxError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Error, notify::Message::from_display(&self.inner))
    }
}

impl notify::Error for TestTxError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Error, notify::Message::from_display(&self.inner))
    }
}
