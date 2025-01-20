use nvimx2::NeovimCtx;
use nvimx2::backend::Backend;
use nvimx2::module::{ApiCtx, Empty, Module};
use nvimx2::neovim::{self, Neovim};
use nvimx2::notify::Name;
use nvimx2::plugin::Plugin;

#[neovim::plugin]
fn mad() -> Mad {
    Mad
}

/// TODO: docs.
struct Mad;

impl Plugin<Neovim> for Mad {
    const COMMAND_NAME: Name = "Mad";
}

impl<B: Backend> Module<B> for Mad {
    const NAME: Name = "mad";

    type Config = Empty;

    fn api(&self, ctx: &mut ApiCtx<B>) {
        let auth = auth::Auth::default();
        let collab = collab2::Collab::default();

        ctx.with_command(auth.login())
            .with_command(auth.logout())
            .with_command(collab.start())
            .with_command(version::EmitVersion::new())
            .with_constant(version::VERSION)
            .with_module(auth)
            .with_module(collab);
    }

    fn on_new_config(&self, _: Self::Config, _: &mut NeovimCtx<B>) {}
}
