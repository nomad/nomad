//! TODO: docs.

use editor::{AccessMut, AgentId, Buffer, Context};

use crate::Neovim;
use crate::buffer::BufferId;
use crate::oxi::api;

/// TODO: docs.
pub trait ContextExt: AccessMut<Neovim> {
    /// TODO: docs.
    fn cmd(&self, cmd: &str) {
        api::command(cmd).expect("couldn't execute command");
    }

    /// Enters insert mode as if "i" was typed in normal mode.
    ///
    /// # Panics
    ///
    /// Panics if Neovim is not in normal mode.
    #[track_caller]
    fn enter_insert_with_i(&self) {
        assert!(api::get_mode().mode == "n", "not in normal mode");
        self.cmd("startinsert");
    }

    /// TODO: docs.
    ///
    /// Note that if Neovim is in insert mode after processing the keys, an
    /// implicit `<Esc>` will be added to put it back in normal mode.
    ///
    /// If you want to enter insert mode, use
    /// [`enter_insert_with_i`](ContextExt::enter_insert_with_i).
    fn feedkeys(&self, keys: &str) {
        let keys = api::replace_termcodes(keys, true, false, true);
        api::feedkeys(&keys, c"x", false);
    }

    /// Shortand for `ctx.cmd("redraw")`.
    fn redraw(&self) {
        self.cmd("redraw");
    }

    /// TODO: docs..
    fn scratch_buffer(&mut self) -> BufferId {
        self.with_mut(|nvim| {
            let scratch_buf_count = nvim.scratch_buffer_count;
            let file_name = format!("scratch-{scratch_buf_count}");
            nvim.scratch_buffer_count += 1;
            let file_path: abs_path::AbsPathBuf = std::env::temp_dir()
                .join(file_name)
                .try_into()
                .expect("it's valid");
            nvim.create_buffer(&file_path, AgentId::UNKNOWN).id()
        })
    }
}

impl<T: AccessMut<Neovim>> ContextExt for T {}

#[doc(hidden)]
pub mod test_macro {
    //! The functions in this module are not part of the crate's public API and
    //! should only be called by the `#[neovim::test]` macro.

    use core::convert::Infallible;
    use core::fmt;
    use core::panic::UnwindSafe;
    use std::panic;
    use std::sync::Arc;

    use editor::Editor;

    use super::*;
    use crate::oxi;

    pub fn sync_test<Out>(
        test_fn: impl FnOnce(&mut Context<Neovim>) -> Out + UnwindSafe,
        test_name: &str,
    ) -> impl FnOnce() -> Out + UnwindSafe
    where
        Out: oxi::IntoResult<()>,
        Out::Error: fmt::Debug,
    {
        || Neovim::new_test(test_name).with_ctx(test_fn)
    }

    pub fn async_test<Out>(
        test_fn: impl AsyncFnOnce(&mut Context<Neovim>) -> Out
        + UnwindSafe
        + 'static,
        test_name: &str,
    ) -> impl FnOnce(oxi::tests::TestTerminator) + UnwindSafe
    where
        Out: oxi::IntoResult<()>,
        Out::Error: fmt::Debug,
    {
        move |terminator| {
            let terminator = Arc::new(terminator);

            panic::set_hook({
                let terminator = terminator.clone();
                Box::new(move |info| {
                    let failure =
                        oxi::tests::TestFailure::<Infallible>::Panic(info);
                    terminator.terminate(Err(failure));
                })
            });

            Neovim::new_test(test_name).with_ctx(move |ctx| {
                ctx.spawn_local(async move |ctx| {
                    let res = test_fn(ctx)
                        .await
                        .into_result()
                        .map_err(oxi::tests::TestFailure::Error);
                    terminator.terminate(res);
                })
                .detach();
            })
        }
    }
}
