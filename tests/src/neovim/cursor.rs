use ed::EditorCtx;
use neovim::Neovim;

use crate::ed::cursor;

#[neovim::test]
fn on_cursor_created(ctx: &mut EditorCtx<Neovim>) {
    cursor::on_cursor_created(ctx);
}
