//! TODO: docs.

use serde::Serialize;
use serde::de::Deserialize;

use crate::backend::{
    Api,
    ApiValue,
    BackendExt,
    BackgroundExecutor,
    Key,
    LocalExecutor,
    MapAccess,
    Value,
};
use crate::module::Module;
use crate::notify::{self, MaybeResult};
use crate::plugin::Plugin;

/// TODO: docs.
pub trait Backend: 'static + Sized {
    /// TODO: docs.
    type Api: Api<Self>;

    /// TODO: docs.
    type LocalExecutor: LocalExecutor;

    /// TODO: docs.
    type BackgroundExecutor: BackgroundExecutor;

    /// TODO: docs.
    type Emitter<'this>: notify::Emitter;

    /// TODO: docs.
    fn api<M: Module<Self>>(&mut self) -> Self::Api;

    /// TODO: docs.
    fn init() -> Self;

    /// TODO: docs.
    fn emitter(&mut self) -> Self::Emitter<'_>;

    /// TODO: docs.
    fn local_executor(&mut self) -> &mut Self::LocalExecutor;

    /// TODO: docs.
    fn background_executor(&mut self) -> &mut Self::BackgroundExecutor;

    /// TODO: docs.
    fn serialize<T>(
        &mut self,
        value: &T,
    ) -> impl MaybeResult<ApiValue<Self>> + use<Self, T>
    where
        T: ?Sized + Serialize;

    /// TODO: docs.
    fn deserialize<'de, T>(
        &mut self,
        value: ApiValue<Self>,
    ) -> impl MaybeResult<T> + use<Self, T>
    where
        T: Deserialize<'de>;

    /// TODO: docs.
    #[allow(unused_variables)]
    fn emit_map_access_error_in_config<P>(
        &mut self,
        config_path: &notify::ModulePath,
        err: <ApiValue<Self> as Value<Self>>::MapAccessError<'_>,
    ) where
        P: Plugin<Self>,
    {
        let module_path = notify::ModulePath::new(P::NAME);
        let source = notify::Source {
            module_path: &module_path,
            action_name: Some(P::CONFIG_FN_NAME),
        };
        self.emit_err(source, err);
    }

    /// TODO: docs.
    #[allow(unused_variables)]
    fn emit_key_as_str_error_in_config<P>(
        &mut self,
        config_path: &notify::ModulePath,
        err: <<<ApiValue<Self> as Value<Self>>::MapAccess<'_> as MapAccess<
            Self,
        >>::Key<'_> as Key<Self>>::AsStrError<'_>,
    ) where
        P: Plugin<Self>,
    {
        let module_path = notify::ModulePath::new(P::NAME);
        let source = notify::Source {
            module_path: &module_path,
            action_name: Some(P::CONFIG_FN_NAME),
        };
        self.emit_err(source, err);
    }
}
