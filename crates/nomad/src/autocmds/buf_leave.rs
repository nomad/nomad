use crate::autocmd::{AutoCommand, AutoCommandEvent, ShouldDetach};
use crate::buffer_id::BufferId;
use crate::ctx::AutoCommandCtx;
use crate::{Action, ActorId};

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
    A: Action<Args = BufLeaveArgs>,
    A::Return: Into<ShouldDetach>,
{
    type Action = A;

    fn into_action(self) -> Self::Action {
        self.action
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

impl From<(ActorId, &AutoCommandCtx<'_>)> for BufLeaveArgs {
    fn from((actor_id, ctx): (ActorId, &AutoCommandCtx<'_>)) -> Self {
        Self { actor_id, buffer_id: BufferId::new(ctx.args().buffer.clone()) }
    }
}
