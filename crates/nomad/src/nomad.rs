use nvim::Dictionary;

use crate::{log, runtime, Api, Command, Config, Module};

/// TODO: docs
pub struct Nomad {
    /// TODO: docs
    api: Dictionary,

    /// TODO: docs
    command: Command,

    /// TODO: docs
    config: Config,
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
        let Self { mut api, command, config } = self;

        command.create();

        api.insert(Config::NAME.as_str(), config.into_function());

        api
    }

    /// TODO: docs
    #[inline]
    pub fn new() -> Self {
        log::init();
        runtime::init();

        log::info!("======== Starting Nomad ========");

        Self::new_default()
    }

    /// TODO: docs
    #[inline]
    fn new_default() -> Self {
        Self {
            api: Dictionary::default(),
            command: Command::default(),
            config: Config::default(),
        }
    }

    /// TODO: docs
    #[inline]
    pub fn with_module<M: Module>(mut self) -> Self {
        let (config, set_config) = runtime::new_input(M::Config::default());

        let Api { commands, functions, module } = M::init(config);

        // Register the module's config.
        self.config.add_module::<M>(set_config);

        // Add the module's commands as sub-commands of the `Nomad` command.
        self.command.add_module::<M>(commands);

        // Add the module's API to the global API.
        self.api.insert(M::NAME.as_str(), functions.into_dict());

        // Spawn a new task that loads the module asynchronously.
        crate::spawn(async move {
            module.run().await;
        })
        .detach();

        self
    }
}
