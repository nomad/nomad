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
pub struct BufAdd<A> {
    action: A,
    buffer_id: Option<BufferId>,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone)]
pub struct BufAddArgs {
    /// The [`ActorId`] that added the buffer.
    pub actor_id: ActorId,
}

impl<A> BufAdd<A> {
    /// TODO: docs.
    pub fn buffer_id(mut self, buffer_id: BufferId) -> Self {
        self.buffer_id = Some(buffer_id);
        self
    }

    /// Creates a new [`BufAdd`] with the given action.
    pub fn new(action: A) -> Self {
        Self { action, buffer_id: None }
    }
}

impl<A> AutoCommand for BufAdd<A>
where
    A: for<'ctx> Action<Args = BufAddArgs, Ctx<'ctx> = BufferCtx<'ctx>>,
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
            let args = BufAddArgs { actor_id };
            let buffer_id = BufferId::new(ctx.args().buffer.clone());
            let buffer_ctx = ctx
                .deref()
                .clone()
                .into_buffer(buffer_id)
                .expect("buffer was just added, so its ID must be valid");
            self.action
                .execute(args, buffer_ctx)
                .into_result()
                .map(Into::into)
                .map_err(Into::into)
        }
    }

    fn on_event(&self) -> AutoCommandEvent {
        AutoCommandEvent::BufAdd
    }

    fn on_buffer(&self) -> Option<BufferId> {
        self.buffer_id
    }

    fn take_actor_id(ctx: &AutoCommandCtx<'_>) -> ActorId {
        let buffer_id = BufferId::new(ctx.args().buffer.clone());
        ctx.with_actor_map(|m| m.take_added_buffer(&buffer_id))
    }
}
