use collab_fs::OsFs;

use super::{Api, ModuleApi, NeovimSpawner};
use crate::Editor;

/// TODO: docs.
#[derive(Default)]
pub struct Neovim {}

impl Editor for Neovim {
    type Fs = OsFs;
    type Api = Api;
    type ModuleApi = ModuleApi;
    type Spawner = NeovimSpawner;

    #[inline]
    fn fs(&self) -> Self::Fs {
        OsFs::new()
    }

    #[inline]
    fn spawner(&self) -> Self::Spawner {
        NeovimSpawner
    }
}
