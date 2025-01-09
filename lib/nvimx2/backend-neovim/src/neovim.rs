use nvimx_core::Plugin;
use nvimx_core::backend::Backend;

use crate::{api, executor, notify, serde, value};

/// TODO: docs.
pub struct Neovim {
    emitter: notify::NeovimEmitter,
    local_executor: executor::NeovimLocalExecutor,
    background_executor: executor::NeovimBackgroundExecutor,
}

impl Backend for Neovim {
    type Api<P: Plugin<Self>> = api::NeovimApi<P>;
    type ApiValue = value::NeovimValue;
    type LocalExecutor = executor::NeovimLocalExecutor;
    type BackgroundExecutor = executor::NeovimBackgroundExecutor;
    type Emitter<'this> = &'this mut notify::NeovimEmitter;
    type SerializeError = serde::NeovimSerializeError;
    type DeserializeError = serde::NeovimDeserializeError;

    #[inline]
    fn init() -> Self {
        Self {
            emitter: notify::NeovimEmitter::default(),
            local_executor: executor::NeovimLocalExecutor::init(),
            background_executor: executor::NeovimBackgroundExecutor::init(),
        }
    }

    #[inline]
    fn api<P: Plugin<Self>>(&mut self) -> Self::Api<P> {
        api::NeovimApi::default()
    }

    #[inline]
    fn emitter(&mut self) -> Self::Emitter<'_> {
        &mut self.emitter
    }

    #[inline]
    fn local_executor(&mut self) -> &mut Self::LocalExecutor {
        &mut self.local_executor
    }

    #[inline]
    fn background_executor(&mut self) -> &mut Self::BackgroundExecutor {
        &mut self.background_executor
    }

    #[inline]
    fn serialize<T: ?Sized + ::serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<Self::ApiValue, Self::SerializeError> {
        serde::serialize(value)
    }

    #[inline]
    fn deserialize<T: ::serde::de::DeserializeOwned>(
        &mut self,
        object: Self::ApiValue,
    ) -> Result<T, Self::DeserializeError> {
        serde::deserialize(object)
    }
}
