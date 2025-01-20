use nvimx2::backend::Backend;
use nvimx2::module::{ApiCtx, Module};
use nvimx2::notify::Name;
use nvimx2::{NeovimCtx, Shared};

use crate::config::Config;
use crate::start::Start;

/// TODO: docs.
#[derive(Default)]
pub struct Collab {
    pub(crate) config: Shared<Config>,
}

impl Collab {
    /// Returns a new instance of the [`Start`] action.
    pub fn start(&self) -> Start {
        self.into()
    }
}

impl<B: Backend> Module<B> for Collab {
    const NAME: Name = "collab";

    type Config = Config;

    fn api(&self, ctx: &mut ApiCtx<B>) {
        ctx.with_function(self.start());
    }

    fn on_new_config(&self, new_config: Self::Config, _: &mut NeovimCtx<B>) {
        self.config.set(new_config);
    }
}
