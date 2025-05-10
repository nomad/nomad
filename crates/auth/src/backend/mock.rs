#![allow(missing_docs)]

use ed::backend::{AgentId, ApiValue, Backend, BaseBackend, BufferId};
use ed::fs::AbsPath;
use ed::notify::MaybeResult;
use ed::{AsyncCtx, EditorCtx};
use serde::{Deserialize, Serialize};

use crate::{AuthBackend, AuthInfos};

pub struct AuthMock<B> {
    inner: B,
}

impl<B> AuthMock<B> {
    pub fn new(inner: B) -> Self {
        Self { inner }
    }
}

impl<B: BaseBackend> AuthBackend for AuthMock<B> {
    type LoginError = core::convert::Infallible;

    #[allow(clippy::manual_async_fn)]
    fn credential_builder(
        _: &mut EditorCtx<Self>,
    ) -> impl Future<Output = Box<keyring::CredentialBuilder>> + Send + 'static
    {
        async move { todo!() }
    }

    async fn login(
        _: &mut AsyncCtx<'_, Self>,
    ) -> Result<AuthInfos, Self::LoginError> {
        todo!()
    }
}

impl<B: BaseBackend> Backend for AuthMock<B> {
    const REINSTATE_PANIC_HOOK: bool = B::REINSTATE_PANIC_HOOK;

    type Api = <B as Backend>::Api;
    type Buffer<'a> = <B as Backend>::Buffer<'a>;
    type BufferId = <B as Backend>::BufferId;
    type Cursor<'a> = <B as Backend>::Cursor<'a>;
    type CursorId = <B as Backend>::CursorId;
    type Fs = <B as Backend>::Fs;
    type LocalExecutor = <B as Backend>::LocalExecutor;
    type BackgroundExecutor = <B as Backend>::BackgroundExecutor;
    type Emitter<'this> = <B as Backend>::Emitter<'this>;
    type EventHandle = <B as Backend>::EventHandle;
    type Selection<'a> = <B as Backend>::Selection<'a>;
    type SelectionId = <B as Backend>::SelectionId;

    type CreateBufferError = <B as Backend>::CreateBufferError;
    type SerializeError = <B as Backend>::SerializeError;
    type DeserializeError = <B as Backend>::DeserializeError;

    fn buffer(&mut self, id: BufferId<Self>) -> Option<Self::Buffer<'_>> {
        self.inner.buffer(id)
    }
    fn buffer_at_path(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        self.inner.buffer_at_path(path)
    }
    fn buffer_ids(&mut self) -> impl Iterator<Item = BufferId<Self>> + use<B> {
        self.inner.buffer_ids()
    }
    async fn create_buffer(
        file_path: &AbsPath,
        agent_id: AgentId,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<Self::BufferId, Self::CreateBufferError> {
        <B as BaseBackend>::create_buffer(file_path, agent_id, ctx).await
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
    fn local_executor(&mut self) -> &mut Self::LocalExecutor {
        self.inner.local_executor()
    }
    fn background_executor(&mut self) -> &mut Self::BackgroundExecutor {
        self.inner.background_executor()
    }
    fn on_buffer_created<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Buffer<'_>, AgentId) + 'static,
    {
        self.inner.on_buffer_created(fun)
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

impl<B> AsMut<B> for AuthMock<B> {
    fn as_mut(&mut self) -> &mut B {
        &mut self.inner
    }
}
