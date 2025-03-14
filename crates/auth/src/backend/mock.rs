#![allow(missing_docs)]

use ed::backend::{ApiValue, Backend, BufferId};
use ed::fs::AbsPath;
use ed::notify::MaybeResult;
use serde::{Deserialize, Serialize};

use crate::AuthBackend;

pub struct AuthMock<B: Backend> {
    inner: B,
}

impl<B: Backend> AuthMock<B> {
    pub fn new(inner: B) -> Self {
        Self { inner }
    }
}

impl<B: Backend> AuthBackend for AuthMock<B> {
    fn credential_store(
        _: &mut ed::EditorCtx<Self>,
    ) -> impl Future<Output = Box<keyring::CredentialBuilder>> + Send + 'static
    {
        async move { todo!() }
    }
}

impl<B: Backend> Backend for AuthMock<B> {
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
