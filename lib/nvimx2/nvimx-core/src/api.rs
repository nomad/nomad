//! TODO: docs.

use crate::command::{CommandArgs, CommandCompletion};
use crate::module::Module;
use crate::{ActionName, Backend, ByteOffset, Plugin, notify};

/// TODO: docs.
pub trait Api<P: Plugin<B>, B: Backend>: 'static + Sized {
    /// TODO: docs.
    type ModuleApi<'a, M: Module<B, Namespace = P>>: ModuleApi<M, B>;

    /// TODO: docs.
    fn add_command<Cmd, CompFun, Comps>(
        &mut self,
        command: Cmd,
        completion_fun: CompFun,
    ) where
        Cmd: FnMut(CommandArgs) + 'static,
        CompFun: FnMut(CommandArgs, ByteOffset) -> Comps + 'static,
        Comps: IntoIterator<Item = CommandCompletion>;

    /// TODO: docs.
    fn with_module<M>(&mut self) -> Self::ModuleApi<'_, M>
    where
        M: Module<B, Namespace = P>;
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
