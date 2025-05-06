//! TODO: docs.

use ed::EditorCtx;
use ed::module::{ApiCtx, Empty, Module};
use ed::notify::Name;
use ed::plugin::Plugin;
use neovim::Neovim;

#[neovim::plugin]
fn mad() -> Mad {
    Mad
}

struct Mad;

impl Plugin<Neovim> for Mad {
    const COMMAND_NAME: Name = "Mad";
}

impl Module<Neovim> for Mad {
    const NAME: Name = "mad";

    type Config = Empty;

    fn api(&self, ctx: &mut ApiCtx<Neovim>) {
        let auth = auth::Auth::default();
        let collab = collab::Collab::from(&auth);

        ctx.with_command(auth.login())
            .with_command(auth.logout())
            .with_command(collab.start())
            .with_command(version::EmitVersion::new())
            .with_constant(version::VERSION)
            .with_module(auth)
            .with_module(collab);
    }

    fn on_init(&self, ctx: &mut EditorCtx<Neovim>) {
        ctx.backend_mut().set_emitter(neovim::notify::detect());
    }

    fn on_new_config(&self, _: Self::Config, _: &mut EditorCtx<Neovim>) {}
}
