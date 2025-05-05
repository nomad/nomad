use core::panic;

use futures_lite::FutureExt;

use crate::AsyncCtx;
use crate::backend::{
    AgentId,
    Backend,
    BackgroundExecutor,
    BufferId,
    LocalExecutor,
    TaskBackground,
    TaskLocal,
};
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
    pub fn buffer(&mut self, buffer_id: BufferId<B>) -> Option<B::Buffer<'_>> {
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
    pub fn focus_buffer_at(
        &mut self,
        path: &AbsPath,
    ) -> Result<Option<B::Buffer<'_>>, core::convert::Infallible> {
        Ok(self.backend_mut().focus_buffer_at(path))
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
        Fun: FnMut(&B::Buffer<'_>) + 'static,
    {
        self.backend_mut().on_buffer_created(fun)
    }

    /// TODO: docs.
    #[inline]
    pub fn new_agent_id(&mut self) -> AgentId {
        self.state.next_agent_id()
    }

    /// TODO: docs.
    #[must_use = "task handles do nothing unless awaited or detached"]
    #[inline]
    pub fn spawn_background<Fut>(
        &mut self,
        fut: Fut,
    ) -> TaskBackground<Fut::Output, B>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let task = self.backend_mut().background_executor().spawn(fut);
        TaskBackground::<_, B>::new(task)
    }

    /// TODO: docs.
    #[must_use = "task handles do nothing unless awaited or detached"]
    #[inline]
    pub fn spawn_local<Out>(
        &mut self,
        fun: impl AsyncFnOnce(&mut AsyncCtx<B>) -> Out + 'static,
    ) -> TaskLocal<Option<Out>, B>
    where
        Out: 'static,
    {
        let fun = async move |ctx: &mut AsyncCtx<B>| {
            let panic_payload =
                match panic::AssertUnwindSafe(fun(ctx)).catch_unwind().await {
                    Ok(ret) => return Some(ret),
                    Err(payload) => payload,
                };

            ctx.state().with_mut(|mut state| {
                state.handle_panic(
                    ctx.namespace(),
                    ctx.plugin_id(),
                    panic_payload,
                );
            });

            None
        };

        self.spawn_local_unprotected(fun)
    }

    /// TODO: docs.
    #[must_use = "task handles do nothing unless awaited or detached"]
    #[inline]
    pub fn spawn_local_unprotected<Out>(
        &mut self,
        fun: impl AsyncFnOnce(&mut AsyncCtx<B>) -> Out + 'static,
    ) -> TaskLocal<Out, B>
    where
        Out: 'static,
    {
        let mut ctx = AsyncCtx::new(
            self.namespace.clone(),
            self.plugin_id,
            self.state.handle(),
        );

        let task = self.local_executor().spawn(async move {
            // Yielding prevents a panic that would occur when:
            //
            // - the local executor immediately polls the future when a new
            //   task is spawned, and
            // - `AsyncCtx::with_ctx()` is called before the first `.await`
            //   point is reached
            //
            // In that case, `with_ctx()` would panic because `State` is
            // already mutably borrowed in this `EditorCtx`.
            //
            // Yielding guarantees that by the time `with_ctx()` is called,
            // the synchronous code in which the `AsyncCtx` was created
            // will have already finished running.
            futures_lite::future::yield_now().await;

            fun(&mut ctx).await
        });

        TaskLocal::<_, B>::new(task)
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

    #[inline]
    pub(crate) fn local_executor(&mut self) -> &mut B::LocalExecutor {
        self.state.local_executor()
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
