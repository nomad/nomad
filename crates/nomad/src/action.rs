use core::marker::PhantomData;

use crate::action_name::ActionName;
use crate::maybe_result::MaybeResult;
use crate::Module;

/// TODO: docs
pub trait Action: 'static {
    /// TODO: docs
    const NAME: ActionName;

    /// TODO: docs
    type Args;

    /// TODO: docs
    type Docs;

    /// TODO: docs
    type Module: Module;

    /// TODO: docs
    //
    // NOTE: remove once we have RTN
    // (https://github.com/rust-lang/rust/issues/109417).
    type Return;

    /// TODO: docs
    fn execute(&mut self, args: Self::Args) -> impl MaybeResult<Self::Return>;

    /// TODO: docs
    fn docs(&self) -> Self::Docs;
}

/// TODO: docs.
pub struct FnAction<Fn, Mod, Args, Ret> {
    fun: Fn,
    module: PhantomData<Mod>,
    args: PhantomData<Args>,
    ret: PhantomData<Ret>,
}

impl<Fun, Mod, Args, Ret> FnAction<Fun, Mod, Args, Ret> {
    /// TODO: docs.
    pub fn new(fun: Fun) -> Self {
        Self { fun, args: PhantomData, module: PhantomData, ret: PhantomData }
    }
}

impl<Fun, Mod, Args, Ret, Res> Action for FnAction<Fun, Mod, Args, Ret>
where
    Fun: FnMut(Args) -> Res + 'static,
    Res: MaybeResult<Ret>,
    Mod: Module,
    Args: 'static,
    Ret: 'static,
{
    // FIXME: use `type_name()` once it's const-stable.
    const NAME: ActionName = ActionName::from_str("{closure}");
    type Args = Args;
    type Docs = ();
    type Module = Mod;
    type Return = Ret;

    fn execute(&mut self, args: Self::Args) -> impl MaybeResult<Self::Return> {
        (self.fun)(args)
    }
    fn docs(&self) {}
}

impl<Fun: Clone, Mod, Args, Ret> Clone for FnAction<Fun, Mod, Args, Ret> {
    fn clone(&self) -> Self {
        Self {
            fun: self.fun.clone(),
            args: PhantomData,
            module: PhantomData,
            ret: PhantomData,
        }
    }
}
