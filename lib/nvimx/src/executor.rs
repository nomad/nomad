//! TODO: docs.

use core::cell::OnceCell;
use core::future::Future;
use core::marker::PhantomData;

use crate::oxi::{self, libuv};

/// A single-threaded executor integrated with the Neovim event loop.
///
/// See the [module-level](crate::executor) documentation for more information.
pub struct Executor<'a> {
    /// The executor state.
    state: OnceCell<ExecutorState>,

    /// A handle to the callback that ticks the executor.
    callback_handle: libuv::AsyncHandle,

    /// A fake lifetime to avoid having to require a `'static` lifetime for the
    /// futures given to [`spawn`](Self::spawn).
    _lifetime: PhantomData<&'a ()>,
}

struct ExecutorState {}

impl<'a> Executor<'a> {
    /// TODO: docs.
    #[inline]
    pub fn spawn<F>(&self, _fut: F)
    where
        F: Future<Output = ()> + 'a,
    {
    }
}
