use core::any::TypeId;
use core::marker::PhantomData;

use crate::backend::{Backend, BackgroundExecutor, TaskBackground};
use crate::notify::{Namespace, NotificationId};
use crate::state::StateHandle;
use crate::{NeovimCtx, notify};

/// TODO: docs.
pub struct AsyncCtx<'a, B: Backend> {
    namespace: Namespace,
    plugin_id: TypeId,
    state: StateHandle<B>,
    _non_static: PhantomData<&'a ()>,
}

impl<B: Backend> AsyncCtx<'_, B> {
    /// TODO: docs.
    #[inline]
    pub fn emit_error<Err>(&mut self, err: Err) -> NotificationId
    where
        Err: notify::Error,
    {
        self.with_ctx(move |ctx| ctx.emit_err(err))
    }

    /// TODO: docs.
    #[inline]
    pub fn spawn_background<Fut>(
        &self,
        fut: Fut,
    ) -> TaskBackground<Fut::Output, B>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let task = self
            .state
            .with_mut(|mut state| state.background_executor().spawn(fut));
        TaskBackground::new(task)
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn with_ctx<Fun, Out>(&self, fun: Fun) -> Out
    where
        Fun: FnOnce(&mut NeovimCtx<B>) -> Out,
    {
        self.state.with_mut(|state| {
            // We're running inside a call to `NeovimCtx::spawn_local()` which
            // is already catching unwinding panics, so we can directly create
            // a `NeovimCtx` here.
            #[allow(deprecated)]
            fun(&mut NeovimCtx::new(&self.namespace, self.plugin_id, state))
        })
    }

    #[inline]
    pub(crate) fn new(
        namespace: Namespace,
        plugin_id: TypeId,
        state: StateHandle<B>,
    ) -> Self {
        Self { namespace, plugin_id, state, _non_static: PhantomData }
    }

    #[inline]
    pub(crate) fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    #[inline]
    pub(crate) fn plugin_id(&self) -> TypeId {
        self.plugin_id
    }

    #[inline]
    pub(crate) fn state(&self) -> &StateHandle<B> {
        &self.state
    }
}
