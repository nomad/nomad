use nvimx_ctx::{
    ActorId,
    AutoCommand,
    AutoCommandCtx,
    AutoCommandEvent,
    BufferCtx,
    BufferId,
    ShouldDetach,
};
use nvimx_diagnostics::DiagnosticMessage;
use nvimx_plugin::{Action, Module};

use crate::cursor_moved::{CursorMoved, CursorMovedArgs};

/// TODO: docs.
pub struct CursorMovedI<A> {
    inner: CursorMoved<A>,
    buffer_id: Option<BufferId>,
}

impl<A> CursorMovedI<A> {
    /// TODO: docs.
    pub fn buffer_id(mut self, buffer_id: BufferId) -> Self {
        self.buffer_id = Some(buffer_id);
        self
    }

    /// Creates a new [`CursorMovedI`] with the given action.
    pub fn new(action: A) -> Self {
        Self { inner: CursorMoved::new(action), buffer_id: None }
    }
}

impl<A> AutoCommand for CursorMovedI<A>
where
    A: for<'ctx> Action<Args = CursorMovedArgs, Ctx<'ctx> = BufferCtx<'ctx>>,
    A::Return: Into<ShouldDetach>,
{
    const MODULE_NAME: Option<&'static str> = Some(A::Module::NAME.as_str());
    const CALLBACK_NAME: Option<&'static str> = Some(A::NAME.as_str());

    fn into_callback(
        self,
    ) -> impl for<'ctx> FnMut(
        ActorId,
        &'ctx AutoCommandCtx<'ctx>,
    ) -> Result<ShouldDetach, DiagnosticMessage> {
        self.inner.into_callback()
    }

    fn on_event(&self) -> AutoCommandEvent {
        AutoCommandEvent::CursorMovedI
    }

    fn on_buffer(&self) -> Option<BufferId> {
        self.buffer_id
    }

    fn take_actor_id(ctx: &AutoCommandCtx<'_>) -> ActorId {
        <CursorMoved<A>>::take_actor_id(ctx)
    }
}
