use ::serde::{Deserialize, Serialize};
use nvimx_core::backend::Backend;
use nvimx_core::module::Module;
use nvimx_core::notify::Namespace;
use nvimx_core::plugin::Plugin;

use crate::{api, executor, notify, serde, value};

/// TODO: docs.
pub struct Neovim {
    emitter: notify::NeovimEmitter,
    local_executor: executor::NeovimLocalExecutor,
    background_executor: executor::NeovimBackgroundExecutor,
}

impl Backend for Neovim {
    type Api = api::NeovimApi;
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
    fn api<M: Module<Self>>(&mut self) -> Self::Api {
        api::NeovimApi::new::<M>()
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
    fn serialize<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<value::NeovimValue, serde::NeovimSerializeError> {
        serde::serialize(value)
    }

    #[inline]
    fn deserialize<'de, T: Deserialize<'de>>(
        &mut self,
        value: value::NeovimValue,
    ) -> Result<T, serde::NeovimDeserializeError> {
        serde::deserialize(value)
    }

    #[inline]
    fn emit_deserialize_error_in_config<P: Plugin<Self>>(
        &mut self,
        config_path: &Namespace,
        namespace: &Namespace,
        mut err: Self::DeserializeError,
    ) {
        err.set_config_path(config_path.clone());
        self.emit_err(namespace, err);
    }
}
