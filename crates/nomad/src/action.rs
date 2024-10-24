use core::marker::PhantomData;
use std::hash::{Hash, Hasher};

use crate::maybe_result::MaybeResult;
use crate::Module;

/// The output of calling [`as_str`](ActionName::as_str) on an [`ActionName`].
pub(crate) type ActionNameStr = &'static str;

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

/// TODO: docs
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionName {
    name: &'static str,
}

impl core::fmt::Debug for ActionName {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ActionName").field(&self.name).finish()
    }
}

impl core::fmt::Display for ActionName {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.name)
    }
}

impl AsRef<str> for ActionName {
    #[inline]
    fn as_ref(&self) -> &str {
        self.name
    }
}

impl ActionName {
    /// TODO: docs
    #[inline]
    pub(crate) fn as_str(&self) -> ActionNameStr {
        self.name
    }

    #[doc(hidden)]
    pub const fn from_str(name: ActionNameStr) -> Self {
        Self { name }
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn id(&self) -> ActionId {
        ActionId::from_action_name(self.name)
    }
}

/// TODO: docs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ActionId(u64);

impl ActionId {
    /// TODO: docs
    #[inline]
    pub(crate) fn from_action_name(name: &str) -> Self {
        let mut hasher = std::hash::DefaultHasher::new();
        name.hash(&mut hasher);
        let hash = hasher.finish();
        Self(hash)
    }
}
