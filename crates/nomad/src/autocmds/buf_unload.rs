use crate::autocmd::{AutoCommand, AutoCommandEvent, ShouldDetach};
use crate::buffer_id::BufferId;
use crate::ctx::AutoCommandCtx;
use crate::{Action, ActorId};

/// TODO: docs.
pub struct BufUnload<A> {
    action: A,
    buffer_id: Option<BufferId>,
}

/// TODO: docs.
pub struct BufUnloadArgs {
    /// The [`ActorId`] that unloaded the buffer.
    pub actor_id: ActorId,

    /// The [`BufferId`] of the buffer that was unloaded.
    pub buffer_id: BufferId,
}

impl<A> BufUnload<A> {
    /// TODO: docs.
    pub fn buffer_id(mut self, buffer_id: BufferId) -> Self {
        self.buffer_id = Some(buffer_id);
        self
    }

    /// Creates a new [`BufUnload`] with the given action.
    pub fn new(action: A) -> Self {
        Self { action, buffer_id: None }
    }
}

impl<A> AutoCommand for BufUnload<A>
where
    A: Action<Args = BufUnloadArgs>,
    A::Return: Into<ShouldDetach>,
{
    type Action = A;

    fn into_action(self) -> Self::Action {
        self.action
    }

    fn on_event(&self) -> AutoCommandEvent {
        AutoCommandEvent::BufUnload
    }

    fn on_buffer(&self) -> Option<BufferId> {
        self.buffer_id
    }

    fn take_actor_id(_: &AutoCommandCtx<'_>) -> ActorId {
        ActorId::unknown()
    }
}

impl From<(ActorId, &AutoCommandCtx<'_>)> for BufUnloadArgs {
    fn from((actor_id, _): (ActorId, &AutoCommandCtx<'_>)) -> Self {
        Self { actor_id, buffer_id: BufferId::current() }
    }
}
