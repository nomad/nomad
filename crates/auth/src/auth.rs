use nomad::config::ConfigReceiver;
use nomad::ctx::NeovimCtx;
use nomad::{module_name, Module, ModuleApi, ModuleName};

use crate::actions::{Login, Logout};

/// TODO: docs.
pub struct Auth {}

impl Module for Auth {
    const NAME: ModuleName = module_name!("auth");

    type Config = ();

    fn init(&self, ctx: NeovimCtx<'_>) -> ModuleApi<Self> {
        let login = Login::new();
        let logout = Logout::new();

        ModuleApi::new(ctx.to_static())
            .command(login.clone())
            .command(logout.clone())
            .function(login)
            .function(logout)
    }

    async fn run(self, _: NeovimCtx<'static>) {}
}

impl From<ConfigReceiver<Self>> for Auth {
    fn from(_: ConfigReceiver<Self>) -> Self {
        Self {}
    }
}
