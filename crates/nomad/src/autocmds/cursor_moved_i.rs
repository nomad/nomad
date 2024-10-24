use crate::autocmd::{AutoCommand, AutoCommandEvent, ShouldDetach};
use crate::autocmds::{CursorMoved, CursorMovedArgs};
use crate::buffer_id::BufferId;
use crate::ctx::AutoCommandCtx;
use crate::{Action, ActorId};

/// TODO: docs.
pub struct CursorMovedI<A> {
    action: A,
    buffer_id: Option<BufferId>,
}

impl<A> CursorMovedI<A> {
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

impl<A> AutoCommand for CursorMovedI<A>
where
    A: Action<Args = CursorMovedArgs>,
    A::Return: Into<ShouldDetach>,
{
    type Action = A;

    fn into_action(self) -> Self::Action {
        self.action
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
