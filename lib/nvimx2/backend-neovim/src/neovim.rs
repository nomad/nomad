use nvimx_core::api::Api;
use nvimx_core::{Backend, Plugin};

use crate::{NeovimBackgroundExecutor, NeovimLocalExecutor, api, notify};

/// TODO: docs.
pub struct Neovim {
    emitter: notify::NeovimEmitter,
}

impl Backend for Neovim {
    type Api<P: Plugin<Self>> = api::NeovimApi<P>;
    type LocalExecutor = NeovimLocalExecutor;
    type BackgroundExecutor = NeovimBackgroundExecutor;
    type Emitter<'a> = &'a mut notify::NeovimEmitter;

    #[inline]
    fn init() -> Self {
        Self { emitter: notify::NeovimEmitter::default() }
    }

    #[inline]
    fn api_builder<P: Plugin<Self>>(
        &mut self,
    ) -> <Self::Api<P> as Api<P, Self>>::Builder<'_> {
        api::NeovimApi::default()
    }

    #[inline]
    fn emitter(&mut self) -> Self::Emitter<'_> {
        &mut self.emitter
    }
}
