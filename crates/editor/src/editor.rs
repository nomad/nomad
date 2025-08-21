use core::fmt::Debug;
use core::hash::Hash;

use abs_path::AbsPath;
use executor::Executor;
use serde::Serialize;
use serde::de::Deserialize;

use crate::notify::{self, Emitter, MaybeResult};
use crate::plugin::Plugin;
use crate::{
    AccessMut,
    AgentId,
    Api,
    ApiValue,
    Buffer,
    Context,
    Cursor,
    Key,
    MapAccess,
    Selection,
    Value,
};

/// TODO: docs.
pub trait Editor: 'static + Sized {
    /// TODO: docs.
    type Api: Api;

    /// TODO: docs.
    type Buffer<'a>: Buffer<
        Editor: Editor<
            BufferId = Self::BufferId,
            EventHandle = Self::EventHandle,
            BufferSaveError = Self::BufferSaveError,
        >,
    >;

    /// TODO: docs.
    type BufferId: Clone + Debug + Eq + Hash;

    /// TODO: docs.
    type Cursor<'a>: Cursor<
        Editor: Editor<
            BufferId = Self::BufferId,
            CursorId = Self::CursorId,
            EventHandle = Self::EventHandle,
        >,
    >;

    /// TODO: docs.
    type CursorId: Clone + Debug + Eq + Hash;

    /// TODO: docs.
    type Fs: fs::Fs;

    /// TODO: docs.
    type Emitter<'a>: notify::Emitter;

    /// TODO: docs.
    type Executor: Executor;

    /// TODO: docs.
    type EventHandle;

    /// TODO: docs.
    type Selection<'a>: Selection<
        Editor: Editor<
            BufferId = Self::BufferId,
            SelectionId = Self::SelectionId,
            EventHandle = Self::EventHandle,
        >,
    >;

    /// TODO: docs.
    type SelectionId: Clone + Debug + Eq + Hash;

    /// TODO: docs.
    type BufferSaveError: Debug + 'static;

    /// TODO: docs.
    type CreateBufferError: Debug;

    /// TODO: docs.
    type SerializeError: Debug + notify::Error;

    /// TODO: docs.
    type DeserializeError: Debug + notify::Error;

    /// TODO: docs.
    fn buffer(&mut self, id: Self::BufferId) -> Option<Self::Buffer<'_>>;

    /// TODO: docs.
    fn buffer_at_path(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>>;

    /// TODO: docs.
    fn create_buffer(
        this: impl AccessMut<Self>,
        file_path: &AbsPath,
        agent_id: AgentId,
    ) -> impl Future<Output = Result<Self::BufferId, Self::CreateBufferError>>;

    /// TODO: docs.
    fn current_buffer(&mut self) -> Option<Self::Buffer<'_>>;

    /// TODO: docs.
    fn for_each_buffer<Fun>(&mut self, fun: Fun)
    where
        Fun: FnMut(Self::Buffer<'_>);

    /// TODO: docs.
    fn cursor(&mut self, id: Self::CursorId) -> Option<Self::Cursor<'_>>;

    /// TODO: docs.
    fn fs(&mut self) -> Self::Fs;

    /// TODO: docs.
    fn emitter(&mut self) -> Self::Emitter<'_>;

    /// TODO: docs.
    fn executor(&mut self) -> &mut Self::Executor;

    /// TODO: docs.
    fn on_buffer_created<Fun>(
        &mut self,
        fun: Fun,
        access: impl AccessMut<Self> + Clone + 'static,
    ) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Buffer<'_>, AgentId) + 'static;

    /// TODO: docs.
    fn on_cursor_created<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Cursor<'_>, AgentId) + 'static;

    /// TODO: docs.
    fn on_selection_created<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Selection<'_>, AgentId) + 'static;

    /// TODO: docs.
    fn reinstate_panic_hook(&self) -> bool;

    /// TODO: docs.
    fn selection(
        &mut self,
        id: Self::SelectionId,
    ) -> Option<Self::Selection<'_>>;

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
    fn with_ctx<R>(self, fun: impl FnOnce(&mut Context<Self>) -> R) -> R {
        fun(&mut Context::from_editor(self))
    }
}
