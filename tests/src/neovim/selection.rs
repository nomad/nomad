use ed::Context;
use futures_util::StreamExt;
use neovim::Neovim;
use neovim::tests::ContextExt;

use crate::ed::selection::SelectionEvent;

#[neovim::test]
async fn charwise_simple(ctx: &mut Context<Neovim>) {
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
    ctx.feedkeys("iHello<Esc>0");

    let mut events = SelectionEvent::new_stream(ctx);

    ctx.feedkeys("v");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Created(0..1));

    ctx.feedkeys("w");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(0..5));

    // We're already at EOF, so trying to select one more character shouldn't
    // do anything.
    ctx.feedkeys("<Right>");

    ctx.feedkeys("<Esc>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Removed);
}
