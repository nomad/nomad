use ed::Backend;
use mock::fs::MockFs;
use mock::{ContextExt, Mock};

mod ed_buffer {
    //! Contains the editor-agnostic buffer tests.

    use super::*;
    use crate::ed::buffer;

    #[test]
    fn fuzz_edits_10e1() {
        Mock::<MockFs>::default().with_ctx(|ctx| {
            ctx.block_on(async |ctx| buffer::fuzz_edits(10, ctx).await)
        });
    }

    #[test]
    fn fuzz_edits_10e2() {
        Mock::<MockFs>::default().with_ctx(|ctx| {
            ctx.block_on(async |ctx| buffer::fuzz_edits(100, ctx).await)
        });
    }

    #[test]
    fn fuzz_edits_10e3() {
        Mock::<MockFs>::default().with_ctx(|ctx| {
            ctx.block_on(async |ctx| buffer::fuzz_edits(1_000, ctx).await)
        });
    }

    #[test]
    fn fuzz_edits_10e4() {
        Mock::<MockFs>::default().with_ctx(|ctx| {
            ctx.block_on(async |ctx| buffer::fuzz_edits(10_000, ctx).await)
        });
    }
}
