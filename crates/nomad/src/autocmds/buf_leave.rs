use crate::autocmd::{AutoCommand, AutoCommandEvent, ShouldDetach};
use crate::buffer_id::BufferId;
use crate::ctx::{AutoCommandCtx, BufferCtx, NeovimCtx};
use crate::maybe_result::MaybeResult;
use crate::{Action, ActionName, ActorId};

/// TODO: docs.
pub struct BufLeave<A> {
    action: A,
    buffer_id: Option<BufferId>,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone)]
pub struct BufLeaveArgs {
    /// The [`ActorId`] that focused the buffer.
    pub actor_id: ActorId,

    /// The [`BufferId`] of the buffer that was left.
    pub buffer_id: BufferId,
}

impl<A> BufLeave<A> {
    /// TODO: docs.
    pub fn buffer_id(mut self, buffer_id: BufferId) -> Self {
        self.buffer_id = Some(buffer_id);
        self
    }

    /// Creates a new [`BufLeave`] with the given action.
    pub fn new(action: A) -> Self {
        Self { action, buffer_id: None }
    }
}

impl<A> AutoCommand for BufLeave<A>
where
    A: for<'ctx> Action<
        BufferCtx<'ctx>,
        Args = BufLeaveArgs,
        Return: Into<ShouldDetach>,
    >,
{
    type Action = Compat<A>;

    fn into_action(self) -> Self::Action {
        Compat(self.action)
    }

    fn on_event(&self) -> AutoCommandEvent {
        AutoCommandEvent::BufLeave
    }

    fn on_buffer(&self) -> Option<BufferId> {
        self.buffer_id
    }

    fn take_actor_id(_: &AutoCommandCtx<'_>) -> ActorId {
        // TODO: Implement this.
        ActorId::unknown()
    }
}

pub struct Compat<A>(A);

impl<'a, A> Action<NeovimCtx<'a>> for Compat<A>
where
    A: Action<BufferCtx<'a>, Args = BufLeaveArgs>,
{
    const NAME: ActionName = A::NAME;
    type Args = A::Args;
    type Docs = A::Docs;
    type Module = A::Module;
    type Return = A::Return;

    fn execute(
        &mut self,
        args: Self::Args,
        ctx: NeovimCtx<'a>,
    ) -> impl MaybeResult<Self::Return> {
        let buffer_ctx = ctx
            .into_buffer(args.buffer_id)
            .expect("autocmd was triggered, so buffer must exist");
        self.0.execute(args, buffer_ctx)
    }
    fn docs(&self) -> Self::Docs {
        self.0.docs()
    }
}

impl From<(ActorId, &AutoCommandCtx<'_>)> for BufLeaveArgs {
    fn from((actor_id, ctx): (ActorId, &AutoCommandCtx<'_>)) -> Self {
        Self { actor_id, buffer_id: BufferId::new(ctx.args().buffer.clone()) }
    }
}
