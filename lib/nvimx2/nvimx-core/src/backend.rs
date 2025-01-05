use core::fmt;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::action_ctx::ModulePath;
use crate::api::Api;
use crate::executor::{BackgroundExecutor, LocalExecutor};
use crate::notify::Emitter;
use crate::{ActionName, Plugin, notify};

/// TODO: docs.
pub trait Backend: 'static + Sized {
    /// TODO: docs.
    type Api<P: Plugin<Self>>: Api<P, Self>;

    /// TODO: docs.
    type ApiValue: Value;

    /// TODO: docs.
    type LocalExecutor: LocalExecutor;

    /// TODO: docs.
    type BackgroundExecutor: BackgroundExecutor;

    /// TODO: docs.
    type Emitter<'this>: notify::Emitter;

    /// TODO: docs.
    type SerializeError: notify::Error + 'static;

    /// TODO: docs.
    type DeserializeError: notify::Error + 'static;

    /// TODO: docs.
    fn api<P: Plugin<Self>>(&mut self) -> Self::Api<P>;

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
    ) -> Result<Self::ApiValue, Self::SerializeError>
    where
        T: ?Sized + Serialize;

    /// TODO: docs.
    fn deserialize<T>(
        &mut self,
        value: Self::ApiValue,
    ) -> Result<T, Self::DeserializeError>
    where
        T: DeserializeOwned;
}

/// TODO: docs.
pub trait Value: Default + 'static {
    /// TODO: docs.
    type MapAccess<'a>: MapAccess<Value = Self>;

    /// TODO: docs.
    type MapAccessError<'a>: notify::Error
    where
        Self: 'a;

    /// TODO: docs.
    fn map_access(
        &mut self,
    ) -> Result<Self::MapAccess<'_>, Self::MapAccessError<'_>>;
}

/// TODO: docs.
pub trait MapAccess {
    /// TODO: docs.
    type Key<'a>: Key
    where
        Self: 'a;

    /// TODO: docs.
    type Value;

    /// TODO: docs.
    fn next_key(&mut self) -> Option<Self::Key<'_>>;

    /// TODO: docs.
    fn take_next_value(&mut self) -> Self::Value;
}

/// TODO: docs.
pub trait Key: fmt::Debug {
    /// TODO: docs.
    type AsStrError<'a>: notify::Error
    where
        Self: 'a;

    /// TODO: docs.
    fn as_str(&self) -> Result<&str, Self::AsStrError<'_>>;
}

/// TODO: docs.
pub(crate) trait BackendExt: Backend {
    #[inline]
    fn emit_action_err<Err: notify::Error>(
        &mut self,
        _module_path: &ModulePath,
        _action_name: &'static ActionName,
        _err: Err,
    ) {
        todo!();
    }

    #[inline]
    fn emit_err<Err: notify::Error>(
        &mut self,
        _module_path: &ModulePath,
        _err: Err,
    ) {
        todo!();
        // let Some(level) = err.to_level() else { return };
        //
        // let notification = notify::Notification {
        //     level,
        //     namespace,
        //     message: err.to_message(),
        //     updates_prev: None,
        // };
        //
        // self.emitter().emit(notification);
    }
}

impl<B: Backend> BackendExt for B {}
