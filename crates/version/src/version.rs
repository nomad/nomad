use nvimx::ctx::NeovimCtx;
use nvimx::plugin::{
    action_name,
    module_name,
    Action,
    ActionName,
    ConfigReceiver,
    Module,
    ModuleApi,
    ModuleName,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// TODO: docs.
#[derive(Copy, Clone)]
pub struct Version;

impl Module for Version {
    const NAME: ModuleName = module_name!("version");

    type Config = ();
    type Plugin = nomad::Nomad;

    fn init(&self, ctx: NeovimCtx<'_>) -> ModuleApi<Self> {
        ModuleApi::new(ctx.to_static()).default_subcommand(Self)
    }

    async fn run(self, _: NeovimCtx<'static>) {}
}

impl Action for Version {
    const NAME: ActionName = action_name!("version");
    type Args = ();
    type Ctx<'a> = NeovimCtx<'a>;
    type Docs = ();
    type Module = Self;
    type Return = ();

    fn execute<'a>(&'a mut self, _: Self::Args, _: NeovimCtx<'a>) {
        nvimx::print!("Nomad v{VERSION}");
    }

    fn docs(&self) -> Self::Docs {}
}

impl From<ConfigReceiver<Self>> for Version {
    fn from(_: ConfigReceiver<Self>) -> Self {
        Self {}
    }
}
