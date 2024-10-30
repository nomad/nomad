use nomad::config::ConfigReceiver;
use nomad::ctx::NeovimCtx;
use nomad::{module_name, Module, ModuleApi, ModuleName, Shared};

use crate::actions::{Join, Start};
use crate::session_status::SessionStatus;

/// TODO: docs.
pub struct Collab {
    config_rx: ConfigReceiver<Self>,
    session_status: Shared<SessionStatus>,
}

impl Module for Collab {
    const NAME: ModuleName = module_name!("collab");

    type Config = ();

    fn init(&self, ctx: NeovimCtx<'_>) -> ModuleApi<Self> {
        let join = Join::new();
        let start = Start::new();

        ModuleApi::new(ctx.to_static())
            .command(join.clone())
            .command(start.clone())
            .function(join)
            .function(start)
    }

    async fn run(self, ctx: NeovimCtx<'static>) {
        todo!()
    }
}

impl From<ConfigReceiver<Self>> for Collab {
    fn from(config_rx: ConfigReceiver<Self>) -> Self {
        Self { config_rx, session_status: Shared::default() }
    }
}
