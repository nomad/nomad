use core::ops::Deref;

use nvimx_common::oxi::api;
use nvimx_common::{ByteOffset, MaybeResult, Point};
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

/// TODO: docs.
pub struct CursorMoved<A> {
    action: A,
    buffer_id: Option<BufferId>,
}

/// TODO: docs.
pub struct CursorMovedArgs {
    /// The [`ActorId`] that moved the cursor.
    pub actor_id: ActorId,

    /// The [`ByteOffset`] the cursor was moved to.
    pub moved_to: ByteOffset,
}

impl<A> CursorMoved<A> {
    /// TODO: docs.
    pub fn buffer_id(mut self, buffer_id: BufferId) -> Self {
        self.buffer_id = Some(buffer_id);
        self
    }

    /// Creates a new [`CursorMoved`] with the given action.
    pub fn new(action: A) -> Self {
        Self { action, buffer_id: None }
    }
}

impl<A> AutoCommand for CursorMoved<A>
where
    A: for<'ctx> Action<Args = CursorMovedArgs, Ctx<'ctx> = BufferCtx<'ctx>>,
    A::Return: Into<ShouldDetach>,
{
    const MODULE_NAME: Option<&'static str> = Some(A::Module::NAME.as_str());
    const CALLBACK_NAME: Option<&'static str> = Some(A::NAME.as_str());

    fn into_callback(
        mut self,
    ) -> impl for<'ctx> FnMut(
        ActorId,
        &'ctx AutoCommandCtx<'ctx>,
    ) -> Result<ShouldDetach, DiagnosticMessage> {
        move |actor_id, ctx| {
            let buffer_id = BufferId::new(ctx.args().buffer.clone());
            let buffer_ctx = ctx
                .deref()
                .reborrow()
                .into_buffer(buffer_id)
                .expect("autocmd was triggered, so buffer must exist");

            let point = {
                let (row, col) = api::Window::current()
                    .get_cursor()
                    .expect("never fails(?)");
                Point { line_idx: row - 1, byte_offset: ByteOffset::new(col) }
            };
            let byte_offset = buffer_ctx.byte_offset_of_point(point);
            let args = CursorMovedArgs { actor_id, moved_to: byte_offset };
            self.action
                .execute(args, buffer_ctx)
                .into_result()
                .map(Into::into)
                .map_err(Into::into)
        }
    }

    fn on_event(&self) -> AutoCommandEvent {
        AutoCommandEvent::CursorMoved
    }

    fn on_buffer(&self) -> Option<BufferId> {
        self.buffer_id
    }

    fn take_actor_id(ctx: &AutoCommandCtx<'_>) -> ActorId {
        let buffer_id = BufferId::new(ctx.args().buffer.clone());
        ctx.with_actor_map(|m| m.take_moved_cursor(&buffer_id))
    }
}
