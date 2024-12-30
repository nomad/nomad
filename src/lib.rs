use nvimx2::module::{ApiCtx, Module, ModuleName};
use nvimx2::neovim::{self, Neovim};
use nvimx2::{NeovimCtx, Plugin};

#[neovim::plugin]
fn mad() -> Mad {
    Mad
}

/// TODO: docs.
struct Mad;

impl Plugin<Neovim> for Mad {}

impl Module<Neovim> for Mad {
    const NAME: &'static ModuleName = ModuleName::new("mad");
    type Config = ();
    type Docs = ();

    fn api<P: Plugin<Neovim>>(&self, _ctx: ApiCtx<'_, '_, Self, P, Neovim>) {
        // ctx.with_module(auth::Auth::new())
        //     .with_module(collab::Collab::new())
        //     .with_module(version::Version::new())
        todo!()
    }

    fn on_config_changed(&mut self, _: (), _: NeovimCtx<'_, Neovim>) {}

    fn docs() {}
}
