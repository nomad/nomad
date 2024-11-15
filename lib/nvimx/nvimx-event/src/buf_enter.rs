use core::marker::PhantomData;
use core::ops::Deref;

use nvimx_action::{Action, ActionName, IntoModuleName};
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

/// TODO: docs.
pub struct BufEnter<A, M> {
    action: BufEnterAction<A, M>,
    buffer_id: Option<BufferId>,
}

/// TODO: docs.
pub struct BufEnterArgs {
    /// The [`ActorId`] that focused the buffer.
    pub actor_id: ActorId,

    /// The [`BufferId`] of the old buffer.
    pub old_buffer_id: BufferId,
}

pub struct BufEnterAction<A, M> {
    action: A,
    module_name: PhantomData<M>,
}

impl<A, M> BufEnter<A, M> {
    /// TODO: docs.
    pub fn buffer_id(mut self, buffer_id: BufferId) -> Self {
        self.buffer_id = Some(buffer_id);
        self
    }

    /// Creates a new [`BufEnter`] with the given action.
    pub fn new(action: A) -> Self {
        Self {
            action: BufEnterAction { action, module_name: PhantomData },
            buffer_id: None,
        }
    }
}

impl<A, M> AutoCommand for BufEnter<A, M>
where
    A: for<'ctx> Action<M, Args = BufEnterArgs, Ctx<'ctx> = BufferCtx<'ctx>>,
    A::Return: Into<ShouldDetach>,
    M: IntoModuleName + 'static,
{
    type Action = BufEnterAction<A, M>;
    type OnModule = M;

    fn into_action(self) -> Self::Action {
        self.action
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

impl<A, M> Action<M> for BufEnterAction<A, M>
where
    A: for<'ctx> Action<M, Args = BufEnterArgs, Ctx<'ctx> = BufferCtx<'ctx>>,
    A::Return: Into<ShouldDetach>,
    M: IntoModuleName + 'static,
{
    const NAME: ActionName = A::NAME;
    type Args = ActorId;
    type Ctx<'ctx> = &'ctx AutoCommandCtx<'ctx>;
    type Docs = A::Docs;
    type Return = A::Return;

    fn execute<'a>(
        &'a mut self,
        args: Self::Args,
        ctx: Self::Ctx<'a>,
    ) -> impl MaybeResult<Self::Return> {
        let old_buffer_id = BufferId::new(ctx.args().buffer.clone());
        let buffer_ctx = ctx
            .deref()
            .clone()
            .into_buffer(BufferId::current())
            .expect("autocmd was triggered, so buffer must exist");
        let args = BufEnterArgs { actor_id: args, old_buffer_id };
        self.action.execute(args, buffer_ctx)
    }

    fn docs(&self) -> Self::Docs {
        self.action.docs()
    }
}
