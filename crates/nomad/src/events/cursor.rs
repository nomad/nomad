use crate::autocmds::{
    BufEnter,
    BufEnterArgs,
    BufLeave,
    BufLeaveArgs,
    CursorMoved,
    CursorMovedArgs,
    CursorMovedI,
};
use crate::ctx::{BufferCtx, NeovimCtx};
use crate::maybe_result::MaybeResult;
use crate::{
    Action,
    ActorId,
    BufferId,
    ByteOffset,
    Event,
    FnAction,
    Shared,
    ShouldDetach,
};

/// TODO: docs.
#[derive(Clone)]
pub struct Cursor {
    /// TODO: docs.
    pub action: CursorAction,
    /// TODO: docs.
    pub buffer_id: BufferId,
    /// TODO: docs.
    pub moved_by: ActorId,
}

/// TODO: docs.
#[derive(Clone, Copy)]
pub enum CursorAction {
    /// The cursor has been moved into the buffer at the given offset.
    Created(ByteOffset),

    /// The cursor has been moved to the given offset.
    Moved(ByteOffset),

    /// The cursor has been moved away from the buffer.
    Removed,
}

/// TODO: docs.
pub struct CursorEvent<A> {
    action: A,
}

impl<A> CursorEvent<A> {
    /// Creates a new [`CursorEvent`] with the given action.
    pub fn new(action: A) -> Self {
        Self { action }
    }
}

impl<A> Event for CursorEvent<A>
where
    A: for<'a> Action<
            BufferCtx<'a>,
            Args = Cursor,
            Return: Into<ShouldDetach>,
        > + Clone,
{
    type Ctx<'a> = BufferCtx<'a>;

    #[allow(clippy::too_many_lines)]
    fn register(self, ctx: Self::Ctx<'_>) {
        let mut action = self.action;
        let buffer_id = ctx.buffer_id();
        let should_detach = Shared::new(ShouldDetach::No);
        let has_just_entered_buf = Shared::new(false);

        let cursor_moved_action =
            FnAction::<_, A::Module, _, ShouldDetach>::new({
                let mut action = action.clone();
                let should_detach = should_detach.clone();
                let just_entered_buf = has_just_entered_buf.clone();
                move |args: CursorMovedArgs, ctx: NeovimCtx<'_>| {
                    let cursor_action = if just_entered_buf.take() {
                        CursorAction::Created(args.moved_to)
                    } else {
                        CursorAction::Moved(args.moved_to)
                    };
                    let cursor = Cursor {
                        action: cursor_action,
                        buffer_id,
                        moved_by: args.actor_id,
                    };
                    let buffer_ctx = ctx
                        .reborrow()
                        .into_buffer(buffer_id)
                        .expect("autocmd was triggered, so buffer must exist");
                    action
                        .execute(cursor, buffer_ctx)
                        .into_result()
                        .map(|ret| {
                            should_detach.set(ret.into());
                            should_detach.get()
                        })
                        .map_err(Into::into)
                }
            });

        CursorMoved::new(cursor_moved_action.clone())
            .buffer_id(buffer_id)
            .register((*ctx).reborrow());

        CursorMovedI::new(cursor_moved_action)
            .buffer_id(buffer_id)
            .register((*ctx).reborrow());

        BufEnter::new(FnAction::<_, A::Module, _, _>::new({
            let should_detach = should_detach.clone();
            move |_: BufEnterArgs, _: BufferCtx<'_>| {
                has_just_entered_buf.set(true);
                should_detach.get()
            }
        }))
        .buffer_id(buffer_id)
        .register((*ctx).reborrow());

        BufLeave::new(FnAction::<_, A::Module, _, ShouldDetach>::new({
            move |args: BufLeaveArgs, ctx: BufferCtx<'_>| {
                action
                    .execute(
                        Cursor {
                            action: CursorAction::Removed,
                            buffer_id,
                            moved_by: args.actor_id,
                        },
                        ctx.reborrow(),
                    )
                    .into_result()
                    .map(|ret| {
                        should_detach.set(ret.into());
                        should_detach.get()
                    })
                    .map_err(Into::into)
            }
        }))
        .buffer_id(buffer_id)
        .register((*ctx).reborrow());
    }
}
