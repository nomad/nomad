use core::marker::PhantomData;
use core::ops::Deref;

use nvimx_action::{Action, ActionName, IntoModuleName};
use nvimx_common::oxi::api;
use nvimx_common::{ByteOffset, MaybeResult, Point};
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
pub struct CursorMoved<A, M> {
    action: CursorMovedAction<A, M>,
    buffer_id: Option<BufferId>,
}

/// TODO: docs.
pub struct CursorMovedArgs {
    /// The [`ActorId`] that moved the cursor.
    pub actor_id: ActorId,

    /// The [`ByteOffset`] the cursor was moved to.
    pub moved_to: ByteOffset,
}

pub struct CursorMovedAction<A, M> {
    action: A,
    module_name: PhantomData<M>,
}

impl<A, M> CursorMoved<A, M> {
    /// TODO: docs.
    pub fn buffer_id(mut self, buffer_id: BufferId) -> Self {
        self.buffer_id = Some(buffer_id);
        self
    }

    /// Creates a new [`CursorMoved`] with the given action.
    pub fn new(action: A) -> Self {
        Self {
            action: CursorMovedAction { action, module_name: PhantomData },
            buffer_id: None,
        }
    }
}

impl<A, M> AutoCommand for CursorMoved<A, M>
where
    A: for<'ctx> Action<
        M,
        Args = CursorMovedArgs,
        Ctx<'ctx> = BufferCtx<'ctx>,
    >,
    A::Return: Into<ShouldDetach>,
    M: IntoModuleName + 'static,
{
    type Action = CursorMovedAction<A, M>;
    type OnModule = M;

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

impl<A, M> Action<M> for CursorMovedAction<A, M>
where
    A: for<'ctx> Action<
        M,
        Args = CursorMovedArgs,
        Ctx<'ctx> = BufferCtx<'ctx>,
    >,
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
        actor_id: Self::Args,
        ctx: Self::Ctx<'a>,
    ) -> impl MaybeResult<Self::Return> {
        let buffer_id = BufferId::new(ctx.args().buffer.clone());
        let buffer_ctx = ctx
            .deref()
            .reborrow()
            .into_buffer(buffer_id)
            .expect("autocmd was triggered, so buffer must exist");

        let point = {
            let (row, col) =
                api::Window::current().get_cursor().expect("never fails(?)");
            Point { line_idx: row - 1, byte_offset: ByteOffset::new(col) }
        };
        let byte_offset = buffer_ctx.byte_offset_of_point(point);
        let args = CursorMovedArgs { actor_id, moved_to: byte_offset };
        self.action.execute(args, buffer_ctx)
    }

    fn docs(&self) -> Self::Docs {
        self.action.docs()
    }
}
