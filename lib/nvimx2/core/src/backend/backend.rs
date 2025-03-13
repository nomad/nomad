use core::fmt::Debug;

use serde::Serialize;
use serde::de::Deserialize;

use crate::backend::{
    Api,
    ApiValue,
    BackgroundExecutor,
    Buffer,
    BufferId,
    Key,
    LocalExecutor,
    MapAccess,
    Value,
};
use crate::notify::{self, Emitter, MaybeResult};
use crate::plugin::Plugin;
use crate::state::StateHandle;
use crate::{EditorCtx, fs};

/// TODO: docs.
pub trait Backend: 'static + Sized {
    /// TODO: docs.
    const REINSTATE_PANIC_HOOK: bool;

    /// TODO: docs.
    type Api: Api;

    /// TODO: docs.
    type Buffer<'a>: Buffer<Id = Self::BufferId>;

    /// TODO: docs.
    type BufferId: Clone + Debug;

    /// TODO: docs.
    type LocalExecutor: LocalExecutor;

    /// TODO: docs.
    type BackgroundExecutor: BackgroundExecutor;

    /// TODO: docs.
    type Fs: fs::Fs;

    /// TODO: docs.
    type Emitter<'this>: notify::Emitter;

    /// TODO: docs.
    type SerializeError: notify::Error;

    /// TODO: docs.
    type DeserializeError: notify::Error;

    /// TODO: docs.
    fn buffer(&mut self, id: BufferId<Self>) -> Option<Self::Buffer<'_>>;

    /// TODO: docs.
    fn buffer_ids(
        &mut self,
    ) -> impl Iterator<Item = BufferId<Self>> + use<Self>;

    /// TODO: docs.
    fn current_buffer(&mut self) -> Option<Self::Buffer<'_>>;

    /// TODO: docs.
    fn fs(&mut self) -> Self::Fs;

    /// TODO: docs.
    fn emitter(&mut self) -> Self::Emitter<'_>;

    /// TODO: docs.
    fn focus_buffer_at(
        &mut self,
        path: &fs::AbsPath,
    ) -> Option<Self::Buffer<'_>>;

    /// TODO: docs.
    fn local_executor(&mut self) -> &mut Self::LocalExecutor;

    /// TODO: docs.
    fn background_executor(&mut self) -> &mut Self::BackgroundExecutor;

    /// TODO: docs.
    fn serialize<T>(
        &mut self,
        value: &T,
    ) -> impl MaybeResult<ApiValue<Self>, Error = Self::SerializeError>
    where
        T: ?Sized + Serialize;

    /// TODO: docs.
    fn deserialize<'de, T>(
        &mut self,
        value: ApiValue<Self>,
    ) -> impl MaybeResult<T, Error = Self::DeserializeError>
    where
        T: Deserialize<'de>;

    /// TODO: docs.
    #[inline]
    fn for_each_buffer<Fun>(&mut self, mut fun: Fun)
    where
        Fun: FnMut(Self::Buffer<'_>),
    {
        self.buffer_ids()
            .for_each(|id| fun(self.buffer(id).expect("buffer exists")))
    }

    /// TODO: docs.
    #[allow(unused_variables)]
    fn emit_deserialize_error_in_config<P>(
        &mut self,
        config_namespace: &notify::Namespace,
        namespace: &notify::Namespace,
        err: Self::DeserializeError,
    ) where
        P: Plugin<Self>,
    {
        self.emit_err(namespace, err);
    }

    /// TODO: docs.
    #[allow(unused_variables)]
    fn emit_map_access_error_in_config<P>(
        &mut self,
        config_namespace: &notify::Namespace,
        namespace: &notify::Namespace,
        err: <ApiValue<Self> as Value>::MapAccessError<'_>,
    ) where
        P: Plugin<Self>,
    {
        self.emit_err(namespace, err);
    }

    /// TODO: docs.
    #[allow(unused_variables)]
    fn emit_key_as_str_error_in_config<P>(
        &mut self,
        config_namespace: &notify::Namespace,
        namespace: &notify::Namespace,
        err: <<<ApiValue<Self> as Value>::MapAccess<'_> as MapAccess>::Key<'_> as Key>::AsStrError<'_>,
    ) where
        P: Plugin<Self>,
    {
        self.emit_err(namespace, err);
    }

    /// TODO: docs.
    #[inline]
    fn emit_err<Err>(
        &mut self,
        namespace: &notify::Namespace,
        err: Err,
    ) -> notify::NotificationId
    where
        Err: notify::Error,
    {
        let (level, message) = err.to_message();

        let notification = notify::Notification {
            level,
            namespace,
            message,
            updates_prev: None,
        };

        self.emitter().emit(notification)
    }

    /// TODO: docs.
    #[inline]
    fn with_ctx<R>(self, fun: impl FnOnce(&mut EditorCtx<Self>) -> R) -> R {
        StateHandle::new(self).with_mut(|mut s| {
            s.with_ctx(
                &notify::Namespace::default(),
                <crate::state::ResumeUnwinding as Plugin<Self>>::id(),
                fun,
            )
            .expect("panics are resumed")
        })
    }
}
