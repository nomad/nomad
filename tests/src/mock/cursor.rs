use ed::backend::Backend;
use mock::Mock;
use mock::fs::MockFs;

use crate::ed::cursor;

#[test]
fn on_cursor_created_1() {
    Mock::<MockFs>::default()
        .with_ctx(|ctx| ctx.with_borrowed(cursor::on_cursor_created_1));
}

#[test]
fn on_cursor_created_2() {
    Mock::<MockFs>::default()
        .with_ctx(|ctx| ctx.with_borrowed(cursor::on_cursor_created_2));
}

#[test]
fn on_cursor_moved_1() {
    Mock::<MockFs>::default()
        .with_ctx(|ctx| ctx.with_borrowed(cursor::on_cursor_moved_1));
}
