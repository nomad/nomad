use nvim::Dictionary;

use crate::command::Command;
use crate::prelude::*;
use crate::{config, log, runtime};

/// TODO: docs
pub struct Nomad {
    /// TODO: docs
    api: Dictionary,

    /// TODO: docs
    command: Command,
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
        let Self { mut api, command } = self;

        command.create();

        api.insert(config::CONFIG_NAME.as_str(), config::config());

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
        Self { api: Dictionary::default(), command: Command::default() }
    }

    /// TODO: docs
    #[inline]
    pub fn with_module<M: Module>(mut self) -> Self {
        let (get_config, set_config) = runtime::input(M::Config::default());

        let api = M::init(get_config);

        // TODO: docs
        config::with_module::<M>(set_config);

        let Api { commands, functions, module } = api;

        // Add the module's commands as sub-commands of the `Nomad` command.
        self.command.add_module::<M>(commands);

        // Add the module's API to the global API.
        let module_api = functions.into_dict();
        self.api.insert(M::NAME.as_str(), module_api);

        // Spawn a new task that loads the module asynchronously.
        runtime::spawn(async move {
            module.run().await;
        })
        .detach();

        self
    }
}
