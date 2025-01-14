use nvimx2::backend::Backend;
use nvimx2::module::{ApiCtx, Module};
use nvimx2::neovim::{self, Neovim};
use nvimx2::notify::Name;
use nvimx2::{NeovimCtx, Plugin};

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

    type Config = ();

    fn api(&self, ctx: &mut ApiCtx<Self, B>) {
        ctx.with_command(auth::Login::new())
            .with_command(auth::Logout::new())
            .with_command(version::EmitVersion::new())
            .with_constant(version::VERSION);
        // .with_module(collab::Collab::new());
    }

    fn on_new_config(&self, _: (), _: &mut NeovimCtx<B>) {}
}
