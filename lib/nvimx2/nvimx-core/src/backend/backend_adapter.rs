use core::ops::DerefMut;

use serde::{Deserialize, Serialize};

use crate::backend::{ApiValue, Backend};
use crate::notify::MaybeResult;

/// TODO: docs.
pub trait BackendAdapter: 'static + DerefMut<Target = Self::Base> {
    /// TODO: docs.
    type Base: Backend;
}

impl<T: BackendAdapter> Backend for T {
    const REINSTATE_PANIC_HOOK: bool =
        <T::Target as Backend>::REINSTATE_PANIC_HOOK;

    type Api = <T::Target as Backend>::Api;
    type Buffer<'a> = <T::Target as Backend>::Buffer<'a>;
    type BufferId = <T::Target as Backend>::BufferId;
    type LocalExecutor = <T::Target as Backend>::LocalExecutor;
    type BackgroundExecutor = <T::Target as Backend>::BackgroundExecutor;
    type Fs = <T::Target as Backend>::Fs;
    type Emitter<'this> = <T::Target as Backend>::Emitter<'this>;
    type SerializeError = <T::Target as Backend>::SerializeError;
    type DeserializeError = <T::Target as Backend>::DeserializeError;

    #[inline]
    fn buffer(&mut self, id: Self::BufferId) -> Option<Self::Buffer<'_>> {
        self.deref_mut().buffer(id)
    }

    #[inline]
    fn buffer_ids(&mut self) -> impl Iterator<Item = Self::BufferId> + use<T> {
        self.deref_mut().buffer_ids()
    }

    #[inline]
    fn current_buffer(&mut self) -> Option<Self::Buffer<'_>> {
        self.deref_mut().current_buffer()
    }

    #[inline]
    fn fs(&mut self) -> Self::Fs {
        self.deref_mut().fs()
    }

    #[inline]
    fn emitter(&mut self) -> Self::Emitter<'_> {
        self.deref_mut().emitter()
    }

    #[inline]
    fn local_executor(&mut self) -> &mut Self::LocalExecutor {
        self.deref_mut().local_executor()
    }

    #[inline]
    fn background_executor(&mut self) -> &mut Self::BackgroundExecutor {
        self.deref_mut().background_executor()
    }

    #[inline]
    fn serialize<V>(
        &mut self,
        value: &V,
    ) -> impl MaybeResult<ApiValue<Self>, Error = Self::SerializeError>
    where
        V: ?Sized + Serialize,
    {
        self.deref_mut().serialize(value)
    }

    #[inline]
    fn deserialize<'de, V>(
        &mut self,
        value: ApiValue<Self>,
    ) -> impl MaybeResult<V, Error = Self::DeserializeError>
    where
        V: Deserialize<'de>,
    {
        self.deref_mut().deserialize(value)
    }
}
