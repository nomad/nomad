use ed::executor::Executor;
use ed::{BorrowState, Context, Editor};

/// TODO: docs.
pub trait ContextExt {
    /// TODO: docs.
    fn block_on<T>(&mut self, fun: impl AsyncFnOnce(&mut Self) -> T) -> T;
}

impl<Ed, Bs: BorrowState> ContextExt for Context<Ed, Bs>
where
    Ed: Editor<Executor: Executor<Runner = crate::executor::Runner>>,
{
    #[inline]
    fn block_on<T>(&mut self, fun: impl AsyncFnOnce(&mut Self) -> T) -> T {
        futures_lite::future::block_on(self.run(fun))
    }
}
