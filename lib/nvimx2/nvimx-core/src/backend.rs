use serde::{Deserializer, Serializer};

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
    type Emitter<'a>: notify::Emitter;

    /// TODO: docs.
    type Serializer: Serializer<Ok = Self::ApiValue, Error: notify::Error + 'static>;

    /// TODO: docs.
    type Deserializer<'de>: Deserializer<'de, Error: notify::Error + 'static>;

    /// TODO: docs.
    fn api<P: Plugin<Self>>(&mut self) -> Self::Api<P>;

    /// TODO: docs.
    fn init() -> Self;

    /// TODO: docs.
    fn emitter(&mut self) -> Self::Emitter<'_>;

    /// TODO: docs.
    fn serializer(&mut self) -> Self::Serializer;

    /// TODO: docs.
    fn deserializer<'de>(
        &mut self,
        value: Self::ApiValue,
    ) -> Self::Deserializer<'de>;
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
                plugin_name: crate::PluginName::new("yoo"),
                module_name: None,
                action_name: None,
            },
            message: err.to_message(),
            updates_prev: None,
        };

        self.emitter().emit(notification);
    }
}
