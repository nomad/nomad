//! TODO: docs.

use crate::command::{CommandArgs, CommandCompletion};
use crate::module::Module;
use crate::{ActionName, Backend, ByteOffset, Plugin, notify};

/// TODO: docs.
pub trait Api<P, B>: 'static + Sized
where
    P: Plugin<B>,
    B: Backend,
{
    /// TODO: docs.
    type ModuleApi<'a, M: Module<B>>: ModuleApi<Self, P, M, B>;

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
    fn as_module(&mut self) -> Self::ModuleApi<'_, P>;
}

/// TODO: docs.
pub trait ModuleApi<A, P, M, B>: Sized
where
    A: Api<P, B>,
    P: Plugin<B>,
    M: Module<B>,
    B: Backend,
{
    /// TODO: docs.
    fn add_function<Fun, Err>(&mut self, fun_name: ActionName, fun: Fun)
    where
        Fun: FnMut(B::ApiValue) -> Result<B::ApiValue, Err> + 'static,
        Err: notify::Error;

    /// TODO: docs.
    fn as_module<M2>(&mut self) -> A::ModuleApi<'_, M2>
    where
        M2: Module<B>;

    /// TODO: docs.
    fn finish(self);
}
