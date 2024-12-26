use nvimx2::neovim::{self, Neovim, NeovimVersion};
use nvimx2::{Plugin, PluginApi, PluginCtx, PluginName};

#[cfg(not(feature = "neovim-nightly"))]
#[nvimx2::plugin(neovim::ZeroDotTen)]
fn mad() -> Mad {
    Mad
}

#[cfg(feature = "neovim-nightly")]
#[nvimx2::plugin(neovim::Nightly)]
fn mad() -> Mad {
    Mad
}

/// TODO: docs.
struct Mad;

impl<V: NeovimVersion> Plugin<Neovim<V>> for Mad {
    const NAME: &'static PluginName = PluginName::new("mad");

    type Docs = ();

    fn api(
        &self,
        _ctx: PluginCtx<'_, Neovim<V>>,
    ) -> PluginApi<Self, Neovim<V>> {
        // PluginApi::new(ctx)
        //     .with_module(auth::Auth::new())
        //     .with_module(collab::Collab::new())
        //     .with_module(version::Version::new())
        todo!();
    }

    fn docs() {}
}
