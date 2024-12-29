use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::api::Api;
use crate::executor::{BackgroundExecutor, LocalExecutor};
use crate::notify::Emitter;
use crate::{Plugin, notify};

/// TODO: docs.
pub trait Backend: 'static + Sized {
    /// TODO: docs.
    type Api<P: Plugin<Self>>: Api<P, Self>;

    /// TODO: docs.
    type ApiValue;

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
pub(crate) trait BackendExt: Backend {
    fn emit_err<Err: notify::Error>(&mut self, err: Err);
}

impl<B: Backend> BackendExt for B {
    #[inline]
    fn emit_err<Err: notify::Error>(&mut self, err: Err) {
        let Some(level) = err.to_level() else { return };

        let notification = notify::Notification {
            level,
            source: notify::Source {
                plugin_name: crate::module::ModuleName::new("yoo"),
                module_name: None,
                action_name: None,
            },
            message: err.to_message(),
            updates_prev: None,
        };

        self.emitter().emit(notification);
    }
}
