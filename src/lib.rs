use nvimx2::module::{Module, ModuleApiCtx, ModuleName};
use nvimx2::neovim::{self, Neovim};
use nvimx2::{NeovimCtx, Plugin};

#[neovim::plugin]
fn mad() -> Mad {
    Mad
}

/// TODO: docs.
struct Mad;

impl Module<Neovim> for Mad {
    const NAME: &'static ModuleName = ModuleName::new("mad");
    type Namespace = Self;
    type Config = ();
    type Docs = ();

    fn api(&self, _ctx: ModuleApiCtx<'_, Self, Neovim>) {
        // ctx.with_module(auth::Auth::new())
        //     .with_module(collab::Collab::new())
        //     .with_module(version::Version::new())
        //     .into_api()
        todo!()
    }

    fn on_config_changed(
        &mut self,
        _: Self::Config,
        _: NeovimCtx<'_, Neovim>,
    ) {
        unreachable!()
    }

    fn docs() {}
}

impl Plugin<Neovim> for Mad {}
