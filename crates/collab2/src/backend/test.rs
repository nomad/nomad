//! TODO: docs.

#![allow(missing_docs)]

use core::convert::Infallible;
use core::error::Error;
use core::fmt;
use core::pin::Pin;
use core::task::{Context, Poll};

use collab_server::SessionId;
use collab_server::message::Message;
use eerie::PeerId;
use futures_util::{Sink, Stream};
use nvimx2::backend::{ApiValue, Backend, Buffer, BufferId};
use nvimx2::notify::{self, MaybeResult};
use nvimx2::{AsyncCtx, fs};
use serde::{Deserialize, Serialize};

use crate::backend::{
    ActionForSelectedSession,
    CollabBackend,
    JoinArgs,
    JoinInfos,
    StartArgs,
    StartInfos,
    default_read_replica,
    default_search_project_root,
};

pub fn message_channel() -> (TestTx, TestRx) {
    let (inner_tx, inner_rx) = flume::unbounded();
    (
        TestTx { inner: inner_tx.into_sink() },
        TestRx { inner: inner_rx.into_stream() },
    )
}

#[allow(clippy::type_complexity)]
pub struct CollabTestBackend<B: Backend> {
    inner: B,
    confirm_start_with: Option<Box<dyn FnMut(&fs::AbsPath) -> bool>>,
    clipboard: Option<SessionId>,
    home_dir_with:
        Option<Box<dyn FnMut(<B as Backend>::Fs) -> fs::AbsPathBuf + Send>>,
    lsp_root_with: Option<
        Box<
            dyn FnMut(<B::Buffer<'_> as Buffer>::Id) -> Option<fs::AbsPathBuf>,
        >,
    >,
    select_session_with: Option<
        Box<
            dyn FnMut(
                &[(fs::AbsPathBuf, SessionId)],
                ActionForSelectedSession,
            ) -> Option<&(fs::AbsPathBuf, SessionId)>,
        >,
    >,
    start_session_with: Option<
        Box<dyn FnMut(StartArgs<'_>) -> Result<StartInfos<Self>, AnyError>>,
    >,
}

pin_project_lite::pin_project! {
    pub struct TestRx {
        #[pin]
        inner: flume::r#async::RecvStream<'static, Message>,
    }
}

pin_project_lite::pin_project! {
    pub struct TestTx {
        #[pin]
        inner: flume::r#async::SendSink<'static, Message>,
    }
}

#[derive(Debug)]
pub struct TestTxError {
    inner: flume::SendError<Message>,
}

#[derive(Debug)]
pub struct AnyError {
    inner: Box<dyn Error>,
}

impl<B: Backend> CollabTestBackend<B> {
    pub fn confirm_start_with(
        mut self,
        fun: impl FnMut(&fs::AbsPath) -> bool + 'static,
    ) -> Self {
        self.confirm_start_with = Some(Box::new(fun) as _);
        self
    }

    pub fn home_dir_with(
        mut self,
        fun: impl FnMut(<B as Backend>::Fs) -> fs::AbsPathBuf + Send + 'static,
    ) -> Self {
        self.home_dir_with = Some(Box::new(fun) as _);
        self
    }

    pub fn lsp_root_with(
        mut self,
        fun: impl FnMut(<B::Buffer<'_> as Buffer>::Id) -> Option<fs::AbsPathBuf>
        + 'static,
    ) -> Self {
        self.lsp_root_with = Some(Box::new(fun) as _);
        self
    }

    pub fn new(inner: B) -> Self {
        Self {
            clipboard: None,
            confirm_start_with: None,
            home_dir_with: None,
            inner,
            lsp_root_with: None,
            select_session_with: None,
            start_session_with: None,
        }
    }

    pub fn select_session_with(
        mut self,
        fun: impl FnMut(
            &[(fs::AbsPathBuf, SessionId)],
            ActionForSelectedSession,
        ) -> Option<&(fs::AbsPathBuf, SessionId)>
        + 'static,
    ) -> Self {
        self.select_session_with = Some(Box::new(fun) as _);
        self
    }

    pub fn start_session_with<E: Error + 'static>(
        mut self,
        mut fun: impl FnMut(StartArgs<'_>) -> Result<StartInfos<Self>, E> + 'static,
    ) -> Self {
        self.start_session_with =
            Some(Box::new(move |args| fun(args).map_err(AnyError::new)));
        self
    }
}

impl AnyError {
    pub fn downcast_ref<T: Error + 'static>(&self) -> Option<&T> {
        self.inner.downcast_ref()
    }

    fn new<E: Error + 'static>(err: E) -> Self {
        Self { inner: Box::new(err) as _ }
    }
}

impl<B: Backend> CollabBackend for CollabTestBackend<B> {
    type ServerRx = TestRx;
    type ServerTx = TestTx;

    type CopySessionIdError = Infallible;
    type HomeDirError = &'static str;
    type JoinSessionError = AnyError;
    type LspRootError = Infallible;
    type ReadReplicaError = default_read_replica::Error<Self>;
    type SearchProjectRootError = default_search_project_root::Error<Self>;
    type ServerRxError = Infallible;
    type ServerTxError = TestTxError;
    type StartSessionError = AnyError;

    async fn confirm_start(
        project_root: &fs::AbsPath,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> bool {
        ctx.with_backend(|this| match &mut this.confirm_start_with {
            Some(fun) => fun(project_root),
            None => true,
        })
    }

    async fn copy_session_id(
        session_id: SessionId,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<(), Self::CopySessionIdError> {
        ctx.with_backend(|this| this.clipboard = Some(session_id));
        Ok(())
    }

    async fn home_dir(
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<fs::AbsPathBuf, Self::HomeDirError> {
        ctx.with_backend(|this| match &mut this.home_dir_with {
            Some(fun) => Ok(fun(this.inner.fs())),
            None => Err("no home directory configured"),
        })
    }

    async fn join_session(
        _: JoinArgs<'_>,
        _: &mut AsyncCtx<'_, Self>,
    ) -> Result<JoinInfos<Self>, Self::JoinSessionError> {
        todo!()
    }

    fn lsp_root(
        buffer_id: <Self::Buffer<'_> as Buffer>::Id,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<Option<fs::AbsPathBuf>, Self::LspRootError> {
        Ok(ctx.with_backend(|this| this.lsp_root_with.as_mut()?(buffer_id)))
    }

    async fn read_replica(
        peer_id: PeerId,
        project_root: &fs::AbsPath,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<eerie::Replica, Self::ReadReplicaError> {
        default_read_replica::read_replica(
            peer_id,
            project_root.to_owned(),
            ctx,
        )
        .await
    }

    async fn search_project_root(
        buffer_id: BufferId<Self>,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<eerie::fs::AbsPathBuf, Self::SearchProjectRootError> {
        default_search_project_root::search(buffer_id, ctx).await
    }

    async fn select_session<'pairs>(
        sessions: &'pairs [(fs::AbsPathBuf, SessionId)],
        action: ActionForSelectedSession,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Option<&'pairs (fs::AbsPathBuf, SessionId)> {
        ctx.with_backend(|this| {
            this.select_session_with.as_mut()?(sessions, action)
        })
    }

    async fn start_session(
        args: StartArgs<'_>,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<StartInfos<Self>, Self::StartSessionError> {
        #[derive(Debug)]
        struct NoStarterSet;

        impl fmt::Display for NoStarterSet {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "no starter set, call \
                     CollabTestBackend::start_session_with() to set one"
                )
            }
        }

        impl Error for NoStarterSet {}

        ctx.with_backend(|this| match this.start_session_with.as_mut() {
            Some(fun) => fun(args),
            None => Err(AnyError::new(NoStarterSet)),
        })
    }
}

impl<B: Backend> Backend for CollabTestBackend<B> {
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

impl<B: Backend + Default> Default for CollabTestBackend<B> {
    fn default() -> Self {
        Self::new(B::default())
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
    type Item = Result<Message, Infallible>;

    fn poll_next(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.project()
            .inner
            .poll_next(ctx)
            .map(|maybe_next| maybe_next.map(Ok))
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

impl notify::Error for TestTxError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Error, notify::Message::from_display(&self.inner))
    }
}
