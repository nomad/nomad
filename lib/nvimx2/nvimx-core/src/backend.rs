//! TODO: docs.

use core::fmt;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::api::Api;
pub(crate) use crate::backend_handle::{BackendHandle, BackendMut};
use crate::executor::{BackgroundExecutor, LocalExecutor};
use crate::notify::{self, Emitter};
use crate::plugin::Plugin;

/// TODO: docs.
pub trait Backend: 'static + Sized {
    /// TODO: docs.
    type Api<P: Plugin<Self>>: Api<P, Self>;

    /// TODO: docs.
    type ApiValue: Value<Self>;

    /// TODO: docs.
    type LocalExecutor: LocalExecutor;

    /// TODO: docs.
    type BackgroundExecutor: BackgroundExecutor;

    /// TODO: docs.
    type Emitter<'this>: notify::Emitter;

    /// TODO: docs.
    type SerializeError: notify::Error<Self>;

    /// TODO: docs.
    type DeserializeError: notify::Error<Self>;

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
pub trait Value<B: Backend>: Default + 'static {
    /// TODO: docs.
    type MapAccess<'a>: MapAccess<B, Value = Self>;

    /// TODO: docs.
    type MapAccessError<'a>: notify::Error<B>
    where
        Self: 'a;

    /// TODO: docs.
    fn map_access(
        &mut self,
    ) -> Result<Self::MapAccess<'_>, Self::MapAccessError<'_>>;
}

/// TODO: docs.
pub trait MapAccess<B: Backend> {
    /// TODO: docs.
    type Key<'a>: Key<B>
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
pub trait Key<B: Backend>: fmt::Debug {
    /// TODO: docs.
    type AsStrError<'a>: notify::Error<B>
    where
        Self: 'a;

    /// TODO: docs.
    fn as_str(&self) -> Result<&str, Self::AsStrError<'_>>;
}

/// TODO: docs.
pub(crate) trait BackendExt: Backend {
    #[inline]
    fn emit_err<P, Err>(&mut self, source: notify::Source, err: Err)
    where
        P: Plugin<Self>,
        Err: notify::Error<Self>,
    {
        let Some((level, message)) = err.to_message::<P>(source) else {
            return;
        };

        let notification = notify::Notification {
            level,
            source,
            message,
            updates_prev: None,
        };

        self.emitter().emit(notification);
    }
}

impl<B: Backend> BackendExt for B {}
