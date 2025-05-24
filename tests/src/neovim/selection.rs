use ed::Context;
use futures_util::StreamExt;
use neovim::Neovim;
use neovim::tests::ContextExt;

use crate::ed::selection::SelectionEvent;

#[neovim::test]
async fn selection_events_1(ctx: &mut Context<Neovim>) {
    ctx.feedkeys("ihello<Esc>b");

    let mut events = SelectionEvent::new_stream(ctx);

    ctx.feedkeys("v");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Created(0..1));

    ctx.feedkeys("<Right>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Moved(0..2));

    ctx.feedkeys("<Esc>");
    assert_eq!(events.next().await.unwrap(), SelectionEvent::Removed);
}
