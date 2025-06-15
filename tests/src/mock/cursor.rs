use ed::Backend;
use mock::fs::MockFs;
use mock::{ContextExt, Mock};

mod ed_cursor {
    //! Contains the editor-agnostic cursor tests.

    use super::*;
    use crate::ed::cursor;

    #[test]
    fn on_cursor_created_1() {
        Mock::<MockFs>::default()
            .with_ctx(|ctx| ctx.block_on(cursor::on_cursor_created_1));
    }

    #[test]
    fn on_cursor_created_2() {
        Mock::<MockFs>::default()
            .with_ctx(|ctx| ctx.block_on(cursor::on_cursor_created_2));
    }

    #[test]
    fn on_cursor_moved_1() {
        Mock::<MockFs>::default()
            .with_ctx(|ctx| ctx.block_on(cursor::on_cursor_moved_1));
    }
}
