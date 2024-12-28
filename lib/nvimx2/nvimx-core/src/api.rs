//! TODO: docs.

use crate::{ActionName, Backend, Module, Plugin, notify};

/// TODO: docs.
pub trait Api<P: Plugin<B>, B: Backend>: 'static + Sized {
    /// TODO: docs.
    type ModuleApi<'a, M: Module<B, Plugin = P>>: ModuleApi<M, B>;

    /// TODO: docs.
    fn with_module<M>(&mut self) -> Self::ModuleApi<'_, M>
    where
        M: Module<B, Plugin = P>;
}

/// TODO: docs.
pub trait ModuleApi<M: Module<B>, B: Backend>: Sized {
    /// TODO: docs.
    fn add_function<Fun, Err>(&mut self, fun_name: &ActionName, fun: Fun)
    where
        Fun: FnMut(B::ApiValue) -> Result<B::ApiValue, Err> + 'static,
        Err: notify::Error;

    /// TODO: docs.
    fn finish(self);
}
