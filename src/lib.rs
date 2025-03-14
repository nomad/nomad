use ed::EditorCtx;
use ed::backend::Backend;
use ed::module::{ApiCtx, Empty, Module};
use ed::neovim::{self, Neovim};
use ed::notify::Name;
use ed::plugin::Plugin;

#[neovim::plugin]
fn mad() -> Mad {
    Mad
}

/// TODO: docs.
struct Mad;

impl Plugin<Neovim> for Mad {
    const COMMAND_NAME: Name = "Mad";
}

impl<B> Module<B> for Mad
where
    B: Backend + auth::AuthBackend + collab::CollabBackend,
{
    const NAME: Name = "mad";

    type Config = Empty;

    fn api(&self, ctx: &mut ApiCtx<B>) {
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

    fn on_new_config(&self, _: Self::Config, _: &mut EditorCtx<B>) {}
}
