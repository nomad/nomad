use core::mem;
use core::time::Duration;

use editor::{
    AgentId,
    Buffer,
    ByteOffset,
    Context,
    Cursor,
    Editor,
    Replacement,
};
use futures_util::stream::{FusedStream, StreamExt};
use futures_util::{FutureExt, select_biased};

use crate::editor::{ContextExt, TestEditor};

pub(crate) async fn on_cursor_created_1(ctx: &mut Context<impl TestEditor>) {
    let agent_id = ctx.new_agent_id();

    let mut events = CursorEvent::new_stream(ctx);

    // Focusing the buffer should create a cursor.
    let buf_id = ctx.create_and_focus_scratch_buffer(agent_id).await;

    match events.next().await.unwrap() {
        CursorEvent::Created(creation) => {
            assert_eq!(creation.buffer_id, buf_id);
            assert_eq!(creation.created_by, agent_id)
        },
        other => panic!("expected Created event, got {other:?}"),
    }
}

pub(crate) async fn on_cursor_created_2(ctx: &mut Context<impl TestEditor>) {
    let agent_id = ctx.new_agent_id();

    let scratch1_id = ctx.create_and_focus_scratch_buffer(agent_id).await;

    let mut events = CursorEvent::new_stream(ctx);

    // Focusing the buffer again shouldn't do anything.
    ctx.with_borrowed(|ctx| ctx.buffer(scratch1_id).unwrap().focus(agent_id));

    // Now create and focus a second buffer, which should create a cursor.
    let scratch2_id = ctx.create_and_focus_scratch_buffer(agent_id).await;

    match events.next().await.unwrap() {
        CursorEvent::Created(creation) => {
            assert_eq!(creation.buffer_id, scratch2_id);
            assert_eq!(creation.created_by, agent_id);
        },
        other => panic!("expected Created event, got {other:?}"),
    }
}

pub(crate) async fn on_cursor_moved_1(ctx: &mut Context<impl TestEditor>) {
    let agent_id = ctx.new_agent_id();

    let mut events = CursorEvent::new_stream(ctx);

    let buf_id = ctx.create_and_focus_scratch_buffer(agent_id).await;

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buf_id.clone()).unwrap();
        buf.edit([Replacement::insertion(0, "Hello world")], agent_id);
    });

    // Drain the event stream.
    let sleep = async_io::Timer::after(Duration::from_millis(500));
    select_biased! {
        _event = events.select_next_some() => {},
        _now = FutureExt::fuse(sleep) => {},
    }

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buf_id.clone()).unwrap();
        buf.for_each_cursor(|mut cursor| {
            cursor.r#move(5, agent_id);
        });
    });

    match events.next().await.unwrap() {
        CursorEvent::Moved(movement) => {
            assert_eq!(movement.byte_offset, 5);
            assert_eq!(movement.buffer_id, buf_id);
            assert_eq!(movement.moved_by, agent_id);
        },
        other => panic!("expected Moved event, got {other:?}"),
    }
}

#[derive(cauchy::Debug, cauchy::PartialEq)]
pub(crate) enum CursorEvent<Ed: Editor> {
    Created(CursorCreation<Ed>),
    Moved(CursorMovement<Ed>),
    Removed(AgentId),
}

#[derive(cauchy::Debug, cauchy::PartialEq)]
pub(crate) struct CursorCreation<Ed: Editor> {
    pub(crate) buffer_id: Ed::BufferId,
    pub(crate) byte_offset: ByteOffset,
    pub(crate) created_by: AgentId,
}

#[derive(cauchy::Debug, cauchy::PartialEq)]
pub(crate) struct CursorMovement<Ed: Editor> {
    pub(crate) buffer_id: Ed::BufferId,
    pub(crate) byte_offset: ByteOffset,
    pub(crate) moved_by: AgentId,
}

impl<Ed: Editor> CursorEvent<Ed> {
    /// Returns a never-ending [`Stream`] of [`CursorEvent`]s.
    #[track_caller]
    pub(crate) fn new_stream(
        ctx: &mut Context<Ed>,
    ) -> impl FusedStream<Item = Self> + Unpin + use<Ed> {
        let (tx, rx) = flume::unbounded();
        let editor = ctx.editor();

        mem::forget(ctx.on_cursor_created(move |cursor, created_by| {
            let event = Self::Created(CursorCreation {
                buffer_id: cursor.buffer_id(),
                byte_offset: cursor.byte_offset(),
                created_by,
            });
            let _ = tx.send(event);

            let tx2 = tx.clone();
            mem::forget(cursor.on_moved(
                move |cursor, moved_by| {
                    let event = Self::Moved(CursorMovement {
                        buffer_id: cursor.buffer_id(),
                        byte_offset: cursor.byte_offset(),
                        moved_by,
                    });
                    let _ = tx2.send(event);
                },
                editor.clone(),
            ));

            let tx2 = tx.clone();
            mem::forget(cursor.on_removed(
                move |_cursor_id, removed_by| {
                    let event = Self::Removed(removed_by);
                    let _ = tx2.send(event);
                },
                editor.clone(),
            ));
        }));

        rx.into_stream()
    }
}
