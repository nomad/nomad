use ed::Context;
use neovim::Neovim;

use crate::ed::cursor;

#[neovim::test]
fn on_cursor_created_1(ctx: &mut Context<Neovim>) {
    ctx.with_borrowed(cursor::on_cursor_created_1);
}

#[neovim::test]
fn on_cursor_created_2(ctx: &mut Context<Neovim>) {
    ctx.with_borrowed(cursor::on_cursor_created_2);
}

#[neovim::test]
fn on_cursor_moved_1(ctx: &mut Context<Neovim>) {
    ctx.with_borrowed(cursor::on_cursor_moved_1);
}
