use core::marker::PhantomData;

use crate::AsyncCtx;
use crate::backend::{Backend, BackendExt, BackendMut};
use crate::executor::{
    BackgroundExecutor,
    LocalExecutor,
    Task,
    TaskBackground,
};
use crate::notify::{self, Emitter, ModulePath, Name, NotificationId, Source};
use crate::plugin::Plugin;

/// TODO: docs.
pub struct NeovimCtx<'a, P, B> {
    backend: BackendMut<'a, B>,
    module_path: &'a ModulePath,
    plugin: PhantomData<P>,
}

impl<'a, P, B> NeovimCtx<'a, P, B>
where
    P: Plugin<B>,
    B: Backend,
{
    /// TODO: docs.
    #[inline]
    pub fn as_mut(&mut self) -> NeovimCtx<'_, P, B> {
        NeovimCtx::new(self.backend.as_mut(), self.module_path)
    }

    /// TODO: docs.
    #[inline]
    pub fn backend_mut(&mut self) -> &mut B {
        self.backend.inner_mut()
    }

    /// TODO: docs.
    #[inline]
    pub fn emit_info(&mut self, message: notify::Message) -> NotificationId {
        self.emit_info_inner(message, None)
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
        Fun: AsyncFnOnce(&mut AsyncCtx<P, B>) + 'static,
    {
        let mut async_ctx = AsyncCtx::<'static, _, _>::new(
            self.backend.handle(),
            self.module_path.clone(),
        );
        self.backend_mut()
            .local_executor()
            .spawn(async move { fun(&mut async_ctx).await })
            .detach();
    }

    #[inline]
    pub(crate) fn emit_err<Err>(&mut self, action_name: Option<Name>, err: Err)
    where
        Err: notify::Error<B>,
    {
        self.backend.emit_err::<P, _>(
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
        self.backend.emitter().emit(notify::Notification {
            level: notify::Level::Info,
            source: Source { module_path: self.module_path, action_name },
            message,
            updates_prev: None,
        })
    }

    #[inline]
    pub(crate) fn module_path(&self) -> &'a ModulePath {
        self.module_path
    }

    #[inline]
    pub(crate) fn new(
        backend: BackendMut<'a, B>,
        module_path: &'a ModulePath,
    ) -> Self {
        Self { backend, module_path, plugin: PhantomData }
    }
}
