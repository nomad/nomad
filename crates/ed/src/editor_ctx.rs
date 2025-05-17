use core::panic;

use crate::backend::{AgentId, Backend};
use crate::fs::AbsPath;
use crate::module::Module;
use crate::notify::{self, Emitter, Namespace, NotificationId};
use crate::plugin::PluginId;
use crate::state::StateMut;

/// TODO: docs.
pub struct EditorCtx<'a, B: Backend> {
    namespace: &'a Namespace,
    plugin_id: PluginId,
    state: StateMut<'a, B>,
}

impl<'a, B: Backend> EditorCtx<'a, B> {
    /// TODO: docs.
    #[inline]
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.state
    }

    /// TODO: docs.
    #[inline]
    pub fn buffer(&mut self, buffer_id: B::BufferId) -> Option<B::Buffer<'_>> {
        self.backend_mut().buffer(buffer_id)
    }

    /// TODO: docs.
    #[inline]
    pub fn buffer_at_path(&mut self, path: &AbsPath) -> Option<B::Buffer<'_>> {
        self.backend_mut().buffer_at_path(path)
    }

    /// TODO: docs.
    #[inline]
    pub fn current_buffer(&mut self) -> Option<B::Buffer<'_>> {
        self.backend_mut().current_buffer()
    }

    /// TODO: docs.
    #[inline]
    pub fn cursor(&mut self, cursor_id: B::CursorId) -> Option<B::Cursor<'_>> {
        self.backend_mut().cursor(cursor_id)
    }

    /// TODO: docs.
    #[inline]
    pub fn emit_error(&mut self, message: notify::Message) -> NotificationId {
        self.emit_message(notify::Level::Error, message)
    }

    /// TODO: docs.
    #[inline]
    pub fn emit_info(&mut self, message: notify::Message) -> NotificationId {
        self.emit_message(notify::Level::Info, message)
    }

    /// TODO: docs.
    #[inline]
    pub fn for_each_buffer(&mut self, fun: impl FnMut(B::Buffer<'_>)) {
        self.backend_mut().for_each_buffer(fun);
    }

    /// TODO: docs.
    #[inline]
    pub fn fs(&mut self) -> B::Fs {
        self.backend_mut().fs()
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn get_module<M>(&self) -> &M
    where
        M: Module<B>,
    {
        match self.try_get_module::<M>() {
            Some(module) => module,
            None => panic!("module {:?} not found", M::NAME),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn on_buffer_created<Fun>(&mut self, fun: Fun) -> B::EventHandle
    where
        Fun: FnMut(&B::Buffer<'_>, AgentId) + 'static,
    {
        self.backend_mut().on_buffer_created(fun)
    }

    /// TODO: docs.
    #[inline]
    pub fn on_cursor_created<Fun>(&mut self, fun: Fun) -> B::EventHandle
    where
        Fun: FnMut(&B::Cursor<'_>, AgentId) + 'static,
    {
        self.backend_mut().on_cursor_created(fun)
    }

    /// TODO: docs.
    #[inline]
    pub fn on_selection_created<Fun>(&mut self, fun: Fun) -> B::EventHandle
    where
        Fun: FnMut(&B::Selection<'_>, AgentId) + 'static,
    {
        self.backend_mut().on_selection_created(fun)
    }

    /// TODO: docs.
    #[inline]
    pub fn new_agent_id(&mut self) -> AgentId {
        self.state.next_agent_id()
    }

    /// TODO: docs.
    #[inline]
    pub fn selection(
        &mut self,
        selection_id: B::SelectionId,
    ) -> Option<B::Selection<'_>> {
        self.backend_mut().selection(selection_id)
    }

    /// TODO: docs.
    #[inline]
    pub fn try_get_module<M>(&self) -> Option<&M>
    where
        M: Module<B>,
    {
        self.state.get_module::<M>()
    }

    #[inline]
    pub(crate) fn emit_err<Err>(&mut self, err: Err) -> NotificationId
    where
        Err: notify::Error,
    {
        self.state.emit_err(self.namespace, err)
    }

    #[inline]
    pub(crate) fn emit_message(
        &mut self,
        level: notify::Level,
        message: notify::Message,
    ) -> NotificationId {
        self.state.emitter().emit(notify::Notification {
            level,
            namespace: self.namespace,
            message,
            updates_prev: None,
        })
    }

    #[doc(hidden)]
    #[deprecated(note = "use `StateMut::with_ctx()` instead")]
    #[inline]
    pub(crate) fn new(
        namespace: &'a Namespace,
        plugin_id: PluginId,
        state: StateMut<'a, B>,
    ) -> Self {
        Self { namespace, plugin_id, state }
    }
}
