use crate::autocmd::{AutoCommand, AutoCommandEvent, ShouldDetach};
use crate::buffer_id::BufferId;
use crate::ctx::AutoCommandCtx;
use crate::{Action, ActorId};

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

    /// The [`BufferId`] of the newly focused buffer.
    pub new_buffer_id: BufferId,
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
    A: Action<Args = BufEnterArgs>,
    A::Return: Into<ShouldDetach>,
{
    type Action = A;

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

impl From<(ActorId, &AutoCommandCtx<'_>)> for BufEnterArgs {
    fn from((actor_id, ctx): (ActorId, &AutoCommandCtx<'_>)) -> Self {
        Self {
            actor_id,
            old_buffer_id: BufferId::new(ctx.args().buffer.clone()),
            new_buffer_id: BufferId::current(),
        }
    }
}
