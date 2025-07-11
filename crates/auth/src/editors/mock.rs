#![allow(missing_docs)]

use abs_path::AbsPath;
use ed::notify::MaybeResult;
use ed::{
    AgentId,
    ApiValue,
    BaseEditor,
    BorrowState,
    Borrowed,
    Context,
    Editor,
};
use serde::{Deserialize, Serialize};

use crate::{AuthEditor, AuthInfos};

pub struct AuthMock<Ed> {
    inner: Ed,
}

impl<Ed> AuthMock<Ed> {
    pub fn new(inner: Ed) -> Self {
        Self { inner }
    }
}

impl<Ed: BaseEditor> AuthEditor for AuthMock<Ed> {
    type LoginError = core::convert::Infallible;

    #[allow(clippy::manual_async_fn)]
    fn credential_builder(
        _: &mut Context<Self, Borrowed>,
    ) -> impl Future<Output = Box<keyring::CredentialBuilder>> + Send + 'static
    {
        async move { todo!() }
    }

    async fn login(
        _: &mut Context<Self>,
    ) -> Result<AuthInfos, Self::LoginError> {
        todo!()
    }
}

impl<Ed: BaseEditor> Editor for AuthMock<Ed> {
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
    ) -> impl Iterator<Item = Self::BufferId> + use<Ed> {
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

impl<Ed> AsMut<Ed> for AuthMock<Ed> {
    fn as_mut(&mut self) -> &mut Ed {
        &mut self.inner
    }
}
