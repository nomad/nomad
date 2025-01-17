use core::any::TypeId;
use core::panic;

use futures_lite::FutureExt;

use crate::AsyncCtx;
use crate::backend::{
    Backend,
    BackgroundExecutor,
    LocalExecutor,
    Task,
    TaskBackground,
};
use crate::module::Module;
use crate::notify::{self, Emitter, Namespace, NotificationId};
use crate::state::StateMut;

/// TODO: docs.
pub struct NeovimCtx<'a, B: Backend> {
    namespace: &'a Namespace,
    plugin_id: TypeId,
    state: StateMut<'a, B>,
}

impl<'a, B: Backend> NeovimCtx<'a, B> {
    /// TODO: docs.
    #[inline]
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.state
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
    pub fn spawn_background<Fut>(&mut self, fut: Fut)
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        TaskBackground::<(), B>::new(
            self.backend_mut().background_executor().spawn(fut),
        )
        .detach();
    }

    /// TODO: docs.
    #[inline]
    pub fn spawn_local<Fun>(&mut self, fun: Fun)
    where
        Fun: AsyncFnOnce(&mut AsyncCtx<B>) + 'static,
    {
        let mut ctx = AsyncCtx::new(
            self.namespace.clone(),
            self.plugin_id,
            self.state.handle(),
        );

        self.local_executor()
            .spawn(async move {
                if let Err(payload) =
                    panic::AssertUnwindSafe(fun(&mut ctx)).catch_unwind().await
                {
                    ctx.state().with_mut(|mut state| {
                        state.handle_panic(
                            ctx.namespace(),
                            ctx.plugin_id(),
                            payload,
                        );
                    })
                }
            })
            .detach();
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
        plugin_id: TypeId,
        state: StateMut<'a, B>,
    ) -> Self {
        Self { namespace, plugin_id, state }
    }
}
