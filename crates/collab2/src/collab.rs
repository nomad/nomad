use nomad::config::ConfigReceiver;
use nomad::ctx::NeovimCtx;
use nomad::{module_name, Module, ModuleApi, ModuleName};

/// TODO: docs.
pub struct Collab {
    config_rx: ConfigReceiver<Self>,
}

impl Module for Collab {
    const NAME: ModuleName = module_name!("collab");

    type Config = ();

    fn init(&self, ctx: NeovimCtx<'_>) -> ModuleApi<Self> {
        todo!()
    }

    async fn run(self, ctx: NeovimCtx<'static>) {
        todo!()
    }
}

impl From<ConfigReceiver<Self>> for Collab {
    fn from(config_rx: ConfigReceiver<Self>) -> Self {
        Self { config_rx }
    }
}
