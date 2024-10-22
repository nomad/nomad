use crate::autocmd::{Autocmd, AutocmdEvent, ShouldDetach};
use crate::ctx::AutocmdCtx;
use crate::neovim::BufferId;
use crate::{Action, ActorId};

/// TODO: docs.
pub struct BufAdd<A> {
    action: A,
}

/// TODO: docs.
pub struct BufAddArgs {
    /// The [`ActorId`] of the actor that added the buffer.
    pub actor_id: ActorId,

    /// The [`BufferId`] of the buffer that was added.
    pub buffer_id: BufferId,
}

impl<A> BufAdd<A> {
    /// Creates a new [`BufAdd`] with the given action.
    pub fn new(action: A) -> Self {
        Self { action }
    }
}

impl<A> Autocmd for BufAdd<A>
where
    A: Action<Args = BufAddArgs> + Clone,
    A::Return: Into<ShouldDetach>,
{
    type Action = A;

    fn into_action(self) -> Self::Action {
        self.action
    }

    fn on_events(&self) -> impl IntoIterator<Item = AutocmdEvent> {
        [AutocmdEvent::BufAdd]
    }

    fn take_actor_id(ctx: &AutocmdCtx<'_>) -> ActorId {
        let buffer_id = BufferId::new(ctx.args().buffer.clone());
        ctx.with_actor_map(|m| m.take_added_buffer(&buffer_id))
    }
}

impl From<(ActorId, &AutocmdCtx<'_>)> for BufAddArgs {
    fn from((actor_id, ctx): (ActorId, &AutocmdCtx<'_>)) -> Self {
        Self { actor_id, buffer_id: BufferId::new(ctx.args().buffer.clone()) }
    }
}
