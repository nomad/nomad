use nvim_oxi::api;

use crate::autocmd::{AutoCommand, AutoCommandEvent, ShouldDetach};
use crate::buffer_id::BufferId;
use crate::ctx::{AutoCommandCtx, NeovimCtx};
use crate::point::Point;
use crate::{Action, ActorId, ByteOffset};

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
    A: for<'a> Action<
        NeovimCtx<'a>,
        Args = CursorMovedArgs,
        Return: Into<ShouldDetach>,
    >,
{
    type Action = A;

    fn into_action(self) -> Self::Action {
        self.action
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

impl From<(ActorId, &AutoCommandCtx<'_>)> for CursorMovedArgs {
    fn from((actor_id, ctx): (ActorId, &AutoCommandCtx<'_>)) -> Self {
        let (row, col) =
            api::Window::current().get_cursor().expect("never fails(?)");

        let point =
            Point { line_idx: row - 1, byte_offset: ByteOffset::new(col) };

        let buffer_id = BufferId::new(ctx.args().buffer.clone());

        let buffer_ctx =
            ctx.as_neovim().reborrow().into_buffer(buffer_id).expect(
                "the CursorMoved event just triggered on the buffer, so it \
                 must exist",
            );

        Self { actor_id, moved_to: buffer_ctx.byte_offset_of_point(point) }
    }
}
