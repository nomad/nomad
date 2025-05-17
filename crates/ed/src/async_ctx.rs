use core::marker::PhantomData;

use abs_path::AbsPath;

use crate::backend::{AgentId, Backend};
use crate::notify::{Namespace, NotificationId};
use crate::plugin::PluginId;
use crate::state::StateHandle;
use crate::{EditorCtx, notify};

/// TODO: docs.
pub struct AsyncCtx<'a, B: Backend> {
    namespace: Namespace,
    plugin_id: PluginId,
    state: StateHandle<B>,
    _non_static: PhantomData<&'a ()>,
}

impl<B: Backend> AsyncCtx<'_, B> {
    /// TODO: docs.
    #[inline]
    pub async fn create_and_focus(
        &mut self,
        _file_path: &AbsPath,
        _agent_id: AgentId,
    ) -> Result<B::BufferId, B::CreateBufferError> {
        todo!();
    }

    /// TODO: docs.
    #[inline]
    pub async fn create_buffer(
        &mut self,
        _file_path: &AbsPath,
        _agent_id: AgentId,
    ) -> Result<B::BufferId, B::CreateBufferError> {
        todo!();
    }

    /// TODO: docs.
    #[inline]
    pub fn emit_err<Err>(&self, err: Err) -> NotificationId
    where
        Err: notify::Error,
    {
        self.with_ctx(move |ctx| ctx.emit_err(err))
    }

    /// TODO: docs.
    #[inline]
    pub fn emit_info(&self, message: notify::Message) -> NotificationId {
        self.emit_message(notify::Level::Info, message)
    }

    /// TODO: docs.
    #[inline]
    pub fn for_each_buffer(&self, fun: impl FnMut(B::Buffer<'_>)) {
        self.with_ctx(move |ctx| ctx.for_each_buffer(fun))
    }

    /// TODO: docs.
    #[inline]
    pub fn fs(&self) -> B::Fs {
        self.with_ctx(|ctx| ctx.fs())
    }

    /// TODO: docs.
    #[inline]
    pub fn new_agent_id(&self) -> AgentId {
        self.with_ctx(|ctx| ctx.new_agent_id())
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn with_backend<Out>(&self, fun: impl FnOnce(&mut B) -> Out) -> Out {
        self.with_ctx(move |ctx| fun(ctx.backend_mut()))
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn with_ctx<Out>(
        &self,
        fun: impl FnOnce(&mut EditorCtx<B>) -> Out,
    ) -> Out {
        self.state.with_mut(|state| {
            // We're running inside a call to `EditorCtx::spawn_local()` which
            // is already catching unwinding panics, so we can directly create
            // a `EditorCtx` here.
            #[allow(deprecated)]
            fun(&mut EditorCtx::new(&self.namespace, self.plugin_id, state))
        })
    }

    #[inline]
    pub(crate) fn emit_message(
        &self,
        level: notify::Level,
        message: notify::Message,
    ) -> NotificationId {
        self.with_ctx(move |ctx| ctx.emit_message(level, message))
    }

    #[inline]
    pub(crate) fn new(
        namespace: Namespace,
        plugin_id: PluginId,
        state: StateHandle<B>,
    ) -> Self {
        Self { namespace, plugin_id, state, _non_static: PhantomData }
    }

    #[inline]
    pub(crate) fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    #[inline]
    pub(crate) fn plugin_id(&self) -> PluginId {
        self.plugin_id
    }

    #[inline]
    pub(crate) fn state(&self) -> &StateHandle<B> {
        &self.state
    }
}
