use nvim::Dictionary;

use crate::prelude::*;
use crate::{config, log, runtime};

/// TODO: docs
pub struct Nomad {
    /// TODO: docs
    api: Dictionary,

    /// TODO: docs
    ctx: Ctx,
}

impl Default for Nomad {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Nomad {
    /// TODO: docs
    #[inline]
    pub fn api(self) -> Dictionary {
        let Self { mut api, .. } = self;
        api.insert(config::CONFIG_NAME.as_str(), config::config());
        api
    }

    /// TODO: docs
    #[inline]
    pub fn new() -> Self {
        log::init();

        log::info!("======== Starting Nomad ========");

        Self::new_default()
    }

    /// TODO: docs
    #[inline]
    fn new_default() -> Self {
        Self { api: Dictionary::default(), ctx: Ctx::default() }
    }

    /// TODO: docs
    #[inline]
    pub fn with_module<M: Module>(mut self) -> Self {
        // Create a new input for the module's config and initialize the
        // module.
        let (api, set_config) = self.ctx.with_init(|init_ctx| {
            let (get, set) = init_ctx.new_input(M::Config::default());
            let api = M::init(get, init_ctx);
            (api, set)
        });

        // TODO: docs
        config::with_module::<M>(set_config, self.ctx.clone());

        let Api { module, functions } = api;

        // Add the module's API to the global API.
        for (name, function) in functions.into_iter(self.ctx.clone()) {
            self.api.insert(name, function);
        }

        // TODO: Create the module's commands.

        let ctx = self.ctx.clone();

        // Spawn a new task that loads the module asynchronously.
        runtime::spawn(async move {
            let _ = ctx.with_set(|_set_ctx| module.run()).await;
        })
        .detach();

        self
    }
}
