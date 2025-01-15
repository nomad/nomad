use crate::AsyncCtx;
use crate::backend::{
    Backend,
    BackgroundExecutor,
    LocalExecutor,
    Task,
    TaskBackground,
};
use crate::module::Module;
use crate::notify::{self, Emitter, ModulePath, Name, NotificationId, Source};
use crate::state::StateMut;

/// TODO: docs.
pub struct NeovimCtx<'a, B> {
    module_path: &'a ModulePath,
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
    pub fn emit_info(&mut self, message: notify::Message) -> NotificationId {
        self.emit_info_inner(message, None)
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
        todo!()
    }

    #[inline]
    pub(crate) fn emit_err<Err>(&mut self, action_name: Option<Name>, err: Err)
    where
        Err: notify::Error,
    {
        self.state.emit_err(
            Source { module_path: self.module_path, action_name },
            err,
        );
    }

    #[inline]
    pub(crate) fn emit_info_inner(
        &mut self,
        message: notify::Message,
        action_name: Option<Name>,
    ) -> NotificationId {
        self.state.emitter().emit(notify::Notification {
            level: notify::Level::Info,
            source: Source { module_path: self.module_path, action_name },
            message,
            updates_prev: None,
        })
    }

    #[inline]
    pub(crate) fn local_executor(&mut self) -> &mut B::LocalExecutor {
        self.state.local_executor()
    }

    #[inline]
    pub(crate) fn new(
        module_path: &'a ModulePath,
        state: StateMut<'a, B>,
    ) -> Self {
        Self { state, module_path }
    }

    #[inline]
    pub(crate) fn to_async(&self) -> AsyncCtx<'static, B> {
        AsyncCtx::new(self.module_path.clone(), self.state.handle())
    }
}
