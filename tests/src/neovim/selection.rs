use core::time::Duration;

use editor::Context;
use futures_util::{FutureExt, StreamExt, select_biased};
use neovim::Neovim;
use neovim::tests::NeovimExt;

use crate::editor::selection::SelectionEvent;

#[neovim::test]
async fn charwise_simple(ctx: &mut Context<Neovim>) {
    ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("ihello<Esc>b");

    let mut events = SelectionEvent::new_stream(ctx);

    ctx.feedkeys("v");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Created(0..1));

    ctx.feedkeys("<Right>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(0..2));

    ctx.feedkeys("<Esc>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Removed);
}

#[neovim::test]
async fn charwise_past_eof(ctx: &mut Context<Neovim>) {
    ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello<Esc>0");

    let mut events = SelectionEvent::new_stream(ctx);

    ctx.feedkeys("v");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Created(0..1));

    ctx.feedkeys("W");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(0..6));

    // We're already at EOF, so trying to select one more character shouldn't
    // do anything.
    ctx.feedkeys("<Right>");

    ctx.feedkeys("<Esc>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Removed);
}

#[neovim::test]
async fn charwise_past_eol(ctx: &mut Context<Neovim>) {
    ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello<CR>World<Esc>0<Up>");

    let mut events = SelectionEvent::new_stream(ctx);

    ctx.feedkeys("v");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Created(0..1));

    ctx.feedkeys("e");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(0..5));

    // In Neovim, trying to select past the end of the line will include the
    // following newline in the selection (if there is one).
    ctx.feedkeys("<Right>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(0..6));

    ctx.feedkeys("<Down>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(0..12));

    ctx.feedkeys("<Esc>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Removed);
}

#[neovim::test]
async fn charwise_multibyte(ctx: &mut Context<Neovim>) {
    ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("i🦀ƒoo🐤<Esc>0");

    let mut events = SelectionEvent::new_stream(ctx);

    ctx.feedkeys("v");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Created(0..4));

    ctx.feedkeys("<Right>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(0..6));
    ctx.feedkeys("<Right>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(0..7));
    ctx.feedkeys("<Right>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(0..8));
    ctx.feedkeys("<Right>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(0..12));

    ctx.feedkeys("<Esc>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Removed);
}

#[neovim::test]
async fn charwise_to_linewise_to_charwise(ctx: &mut Context<Neovim>) {
    ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello<CR>World<Esc><Left>");

    let mut events = SelectionEvent::new_stream(ctx);

    ctx.feedkeys("v");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Created(9..10));
    ctx.feedkeys("2<Left>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(7..10));
    ctx.feedkeys("<Up>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(1..10));

    // Switch to linewise visual mode.
    ctx.feedkeys("<S-v>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(0..12));

    // Switch back to charwise visual mode.
    ctx.feedkeys("v");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(1..10));

    ctx.feedkeys("v");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Removed);
}

#[neovim::test]
async fn charwise_to_blockwise_to_charwise(ctx: &mut Context<Neovim>) {
    ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello<CR>World<Esc><Left>");

    let mut events = SelectionEvent::new_stream(ctx);

    ctx.feedkeys("v");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Created(9..10));
    ctx.feedkeys("2<Left>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(7..10));
    ctx.feedkeys("<Up>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(1..10));

    // Switch to blockwise visual mode. Because we don't yet support it, it
    // should be as if we ended visual mode.
    ctx.feedkeys("<C-v>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Removed);

    // Switch back to charwise visual mode.
    ctx.feedkeys("v");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Created(1..10));

    ctx.feedkeys("v");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Removed);
}

#[neovim::test]
async fn linewise_simple(ctx: &mut Context<Neovim>) {
    ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello<Esc>2<Left>");

    let mut events = SelectionEvent::new_stream(ctx);

    ctx.feedkeys("<S-v>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Created(0..6));

    ctx.feedkeys("<Esc>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Removed);
}

#[neovim::test]
async fn blockwise_is_ignored(ctx: &mut Context<Neovim>) {
    ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello<CR>2<Left>");

    let mut events = SelectionEvent::new_stream(ctx);

    ctx.feedkeys("<C-v>");

    let sleep = async_io::Timer::after(Duration::from_millis(500));

    select_biased! {
        _event = events.select_next_some() => {
            panic!(
                "blockwise selections are not currently supported, we \
                 shouldn't emit an event!"
            )
        },
        _now = FutureExt::fuse(sleep) => {},
    }
}
