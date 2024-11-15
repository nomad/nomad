use core::future::Future;
use core::pin::Pin;

use nvimx_common::{oxi, MaybeResult};
use nvimx_ctx::NeovimCtx;
use nvimx_diagnostics::{DiagnosticSource, Level};

use crate::command::Command;
use crate::config::Setup;
use crate::module::Module;
use crate::plugin::Plugin;

/// TODO: docs.
pub struct PluginCtx<P: Plugin> {
    api: oxi::Dictionary,
    command: Command,
    neovim_ctx: NeovimCtx<'static>,
    plugin: P,
    run: Vec<Pin<Box<dyn Future<Output = ()>>>>,
    setup: Setup,
}

impl<P: Plugin> PluginCtx<P> {
    /// TODO: docs.
    pub fn init(plugin: P) -> Self {
        Self {
            api: oxi::Dictionary::default(),
            command: Command::new::<P>(),
            neovim_ctx: NeovimCtx::init(P::AUGROUP_NAME, P::NAMESPACE_NAME),
            plugin,
            run: Vec::new(),
            setup: Setup::default(),
        }
    }

    /// TODO: docs.
    pub fn with_module<M>(mut self) -> Self
    where
        M: Module<Plugin = P>,
    {
        let config_rx = self.setup.add_module::<M>();
        let module = M::from(config_rx);
        let module_api = module.init(self.neovim_ctx.reborrow());
        self.api.insert(M::NAME.as_str(), module_api.dictionary);
        self.command.add_module(module_api.commands);
        self.run.push({
            let neovim_ctx = self.neovim_ctx.clone();
            Box::pin(async move {
                if let Err(err) = module.run(neovim_ctx).await.into_result() {
                    let mut source = DiagnosticSource::new();
                    source.push_segment(M::NAME.as_str());
                    err.into().emit(Level::Error, source);
                }
            })
        });
        self
    }
}

impl<P: Plugin> oxi::lua::Pushable for PluginCtx<P> {
    unsafe fn push(
        mut self,
        state: *mut oxi::lua::ffi::State,
    ) -> Result<i32, oxi::lua::Error> {
        crate::log::init(&self.plugin.log_dir());

        // Start each module's event loop.
        for fut in self.run.drain(..) {
            self.neovim_ctx.spawn(|_| fut).detach();
        }

        self.command.create();

        let setup = oxi::Function::from_fn(self.setup.into_fn());
        self.api.insert(Setup::NAME, setup);
        self.api.push(state)
    }
}
