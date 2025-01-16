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
pub struct NeovimCtx<'a, B> {
    namespace: &'a Namespace,
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
    pub fn emit_err<Err>(&mut self, err: Err)
    where
        Err: notify::Error,
    {
        self.state.emit_err(self.namespace, err);
    }

    /// TODO: docs.
    #[inline]
    pub fn emit_info(&mut self, message: notify::Message) -> NotificationId {
        self.state.emitter().emit(notify::Notification {
            level: notify::Level::Info,
            namespace: self.namespace,
            message,
            updates_prev: None,
        })
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
    pub fn spawn_background<Fut>(
        &mut self,
        fut: Fut,
    ) -> TaskBackground<Fut::Output, B>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        TaskBackground::new(
            self.backend_mut().background_executor().spawn(fut),
        )
    }

    /// TODO: docs.
    #[inline]
    pub fn spawn_local<Fun>(&mut self, fun: Fun)
    where
        Fun: AsyncFnOnce(&mut AsyncCtx<B>) + 'static,
    {
        let mut async_ctx = self.to_async();
        self.local_executor()
            .spawn(async move { fun(&mut async_ctx).await })
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
    pub(crate) fn local_executor(&mut self) -> &mut B::LocalExecutor {
        self.state.local_executor()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `StateMut::with_ctx()` instead")]
    #[inline]
    pub(crate) fn new(
        namespace: &'a Namespace,
        state: StateMut<'a, B>,
    ) -> Self {
        Self { namespace, state }
    }

    #[inline]
    pub(crate) fn to_async(&self) -> AsyncCtx<'static, B> {
        AsyncCtx::new(self.namespace.clone(), self.state.handle())
    }
}
