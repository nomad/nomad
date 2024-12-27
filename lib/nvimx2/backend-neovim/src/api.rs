//! TODO: docs.

use core::marker::PhantomData;

use nvimx_core::api::{Api, ApiBuilder, ModuleApi, ModuleApiBuilder};
use nvimx_core::{Module, Plugin};

use crate::Neovim;
use crate::oxi::Dictionary;

/// TODO: docs.
pub struct NeovimApi<P> {
    dict: Dictionary,
    _phantom: PhantomData<P>,
}

/// TODO: docs.
pub struct NeovimModuleApi<M> {
    dict: Dictionary,
    _phantom: PhantomData<M>,
}

impl<P> Api<P, Neovim> for NeovimApi<P>
where
    P: Plugin<Neovim>,
{
    type Builder<'a> = Self;
    type ModuleApi<M: Module<Neovim, Plugin = P>> = NeovimModuleApi<M>;
}

impl<P> ApiBuilder<NeovimApi<P>, P, Neovim> for NeovimApi<P>
where
    P: Plugin<Neovim>,
{
    #[inline]
    fn add_module<M>(&mut self, module_api: NeovimModuleApi<M>)
    where
        M: Module<Neovim, Plugin = P>,
    {
        self.dict.insert(M::NAME.as_str(), module_api.dict);
        todo!();
    }

    #[inline]
    fn module_builder<M>(&mut self) -> NeovimModuleApi<M>
    where
        M: Module<Neovim, Plugin = P>,
    {
        NeovimModuleApi::default()
    }

    #[inline]
    fn build(self) -> NeovimApi<P> {
        self
    }
}

impl<M> ModuleApi<M, Neovim> for NeovimModuleApi<M>
where
    M: Module<Neovim>,
{
    type Builder<'a> = Self;
}

impl<M> ModuleApiBuilder<NeovimModuleApi<M>, M, Neovim> for NeovimModuleApi<M>
where
    M: Module<Neovim>,
{
    #[inline]
    fn build(self) -> NeovimModuleApi<M> {
        self
    }
}

impl<P> Default for NeovimApi<P>
where
    P: Plugin<Neovim>,
{
    #[inline]
    fn default() -> Self {
        Self { dict: Dictionary::default(), _phantom: PhantomData }
    }
}

impl<M> Default for NeovimModuleApi<M>
where
    M: Module<Neovim>,
{
    #[inline]
    fn default() -> Self {
        Self { dict: Dictionary::default(), _phantom: PhantomData }
    }
}

impl<P> From<NeovimApi<P>> for Dictionary
where
    P: Plugin<Neovim>,
{
    #[inline]
    fn from(_api: NeovimApi<P>) -> Self {
        todo!();
    }
}
