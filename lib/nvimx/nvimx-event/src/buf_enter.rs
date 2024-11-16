use core::ops::Deref;

use nvimx_common::MaybeResult;
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
pub struct BufEnter<A> {
    action: A,
    buffer_id: Option<BufferId>,
}

/// TODO: docs.
pub struct BufEnterArgs {
    /// The [`ActorId`] that focused the buffer.
    pub actor_id: ActorId,

    /// The [`BufferId`] of the old buffer.
    pub old_buffer_id: BufferId,
}

impl<A> BufEnter<A> {
    /// TODO: docs.
    pub fn buffer_id(mut self, buffer_id: BufferId) -> Self {
        self.buffer_id = Some(buffer_id);
        self
    }

    /// Creates a new [`BufEnter`] with the given action.
    pub fn new(action: A) -> Self {
        Self { action, buffer_id: None }
    }
}

impl<A> AutoCommand for BufEnter<A>
where
    A: for<'ctx> Action<Args = BufEnterArgs, Ctx<'ctx> = BufferCtx<'ctx>>,
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
            let old_buffer_id = BufferId::new(ctx.args().buffer.clone());
            let buffer_ctx = ctx
                .deref()
                .clone()
                .into_buffer(BufferId::current())
                .expect("autocmd was triggered, so buffer must exist");
            let args = BufEnterArgs { actor_id, old_buffer_id };
            self.action
                .execute(args, buffer_ctx)
                .into_result()
                .map(Into::into)
                .map_err(Into::into)
        }
    }

    fn on_event(&self) -> AutoCommandEvent {
        AutoCommandEvent::BufEnter
    }

    fn on_buffer(&self) -> Option<BufferId> {
        self.buffer_id
    }

    fn take_actor_id(ctx: &AutoCommandCtx<'_>) -> ActorId {
        let buffer_id = BufferId::new(ctx.args().buffer.clone());
        ctx.with_actor_map(|m| m.take_focused_buffer(&buffer_id))
    }
}
