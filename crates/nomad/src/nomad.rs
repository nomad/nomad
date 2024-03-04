use crate::ctx::Ctx;
use crate::nvim::{Dictionary, Function};
use crate::prelude::{EnableConfig, Module};
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
        api.insert("config", config::config());
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
        let (module, set_config) = self.ctx.with_init(|init_ctx| {
            let default_config = EnableConfig::<M>::default();
            let (get, set) = init_ctx.new_input(default_config);
            let module = M::init(get, init_ctx);
            (module, set)
        });

        // TODO: docs
        config::with_module::<M>(set_config, &self.ctx);

        // Add the module's API to the global API.
        self.api.insert(M::NAME.as_str(), module_api(&module, &self.ctx));

        // TODO: Create the module's commands.
        for _command in module.commands() {}

        let ctx = self.ctx.clone();

        // Spawn a new task that loads the module asynchronously.
        runtime::spawn(async move {
            let _ = ctx.with_set(|_set_ctx| module.load()).await;
        })
        .detach();

        self
    }
}

/// TODO: docs
#[inline]
fn module_api<M: Module>(module: &M, ctx: &Ctx) -> Dictionary {
    let mut dict = Dictionary::new();

    for (action_name, action) in module.api().into_iter() {
        let ctx = ctx.clone();

        let function = move |object| {
            ctx.with_set(|set_ctx| action(object, set_ctx));
            Ok::<_, core::convert::Infallible>(())
        };

        dict.insert(action_name.as_str(), Function::from_fn(function));
    }

    dict
}
