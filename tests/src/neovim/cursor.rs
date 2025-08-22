use core::mem;
use core::time::Duration;

use editor::{ByteOffset, Context};
use futures_util::stream::{FusedStream, StreamExt};
use futures_util::{FutureExt, select_biased};
use neovim::Neovim;
use neovim::buffer::BufferId;
use neovim::tests::NeovimExt;

#[neovim::test]
async fn normal_to_insert_with_i(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("ihello<Esc>");

    let mut offsets = ByteOffset::new_stream(buffer_id, ctx);

    // The offset of a block cursor is set to its left-hand side, so entering
    // insert mode with "i" shouldn't change the offset.
    ctx.enter_insert_with_i();

    let sleep = async_io::Timer::after(Duration::from_millis(500));
    select_biased! {
        offset = offsets.select_next_some() => {
            panic!("expected no offsets, got {offset:?}");
        },
        _now = FutureExt::fuse(sleep) => {},
    }
}

#[neovim::test]
async fn normal_to_insert_with_a(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("ihello<Esc>");

    let mut offsets = ByteOffset::new_stream(buffer_id, ctx);

    // The offset of a block cursor is set to its left-hand side, so entering
    // insert mode with "a" should move the offset to its right side.
    ctx.feedkeys("a<Esc>");

    assert_eq!(offsets.next().await.unwrap(), 5);
}

#[neovim::test]
async fn insert_to_normal(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("ihello<Esc>");

    // The cursor is now between the second "l" and the "o".
    ctx.enter_insert_with_i();

    let mut offsets = ByteOffset::new_stream(buffer_id, ctx);

    // When we switch from insert to normal mode, the cursor is moved on top
    // of the second "l", which is at offset 3.
    ctx.feedkeys("<Esc>");

    assert_eq!(offsets.next().await.unwrap(), 3);
}

trait ByteOffsetExt {
    /// Returns a never-ending stream of [`ByteOffset`]s on the buffer
    /// with the given ID corresponding to the cursor positions.
    fn new_stream(
        buffer_id: BufferId,
        ctx: &mut Context<Neovim>,
    ) -> impl FusedStream<Item = ByteOffset> + Unpin + use<Self> {
        use editor::{Buffer, Cursor};

        let (tx, rx) = flume::unbounded();
        let editor = ctx.editor();

        ctx.with_borrowed(|ctx| {
            ctx.buffer(buffer_id).unwrap().for_each_cursor(
                move |mut cursor| {
                    let tx2 = tx.clone();
                    mem::forget(cursor.on_moved(
                        move |cursor, _moved_by| {
                            let _ = tx2.send(cursor.byte_offset());
                        },
                        editor.clone(),
                    ));
                },
            )
        });

        rx.into_stream()
    }
}

impl ByteOffsetExt for ByteOffset {}

mod ed_cursor {
    //! Contains the editor-agnostic cursor tests.

    use super::*;
    use crate::editor::cursor;

    #[neovim::test]
    async fn on_cursor_created_1(ctx: &mut Context<Neovim>) {
        cursor::on_cursor_created_1(ctx).await;
    }

    #[neovim::test]
    async fn on_cursor_created_2(ctx: &mut Context<Neovim>) {
        cursor::on_cursor_created_2(ctx).await;
    }

    #[neovim::test]
    async fn on_cursor_moved_1(ctx: &mut Context<Neovim>) {
        cursor::on_cursor_moved_1(ctx).await;
    }
}
