use super::Neovim;
use crate::Module;

/// TODO: docs.
pub struct NeovimModuleApi<M: Module<Neovim>> {
    module: M,
}
