use crate::{Backend, Command, Function, Module};

/// TODO: docs.
pub struct ModuleApi<M, B> {
    module: M,
    backend: B,
}

impl<M, B> ModuleApi<M, B>
where
    M: Module<B>,
    B: Backend,
{
    /// TODO: docs.
    #[inline]
    pub fn with_command<C>(self, cmd: C) -> Self
    where
        C: Command<B, Module = M>,
    {
        todo!();
    }

    /// TODO: docs.
    #[inline]
    pub fn with_default_command<C>(self, cmd: C) -> Self
    where
        C: Command<B, Module = M>,
    {
        todo!();
    }

    /// TODO: docs.
    #[inline]
    pub fn with_default_function<F>(self, fun: F) -> Self
    where
        F: Function<B, Module = M>,
    {
        todo!();
    }

    /// TODO: docs.
    #[inline]
    pub fn with_function<F>(self, fun: F) -> Self
    where
        F: Function<B, Module = M>,
    {
        todo!();
    }
}
