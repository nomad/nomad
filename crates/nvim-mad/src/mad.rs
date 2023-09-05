use std::collections::HashMap;

use common::oxi::{Dictionary, Function, Object};
use common::*;

use crate::config;
use crate::runtime::{self, MadRuntime};

/// TODO: docs
pub(crate) struct Mad {
    /// TODO: docs
    api: HashMap<&'static str, Dictionary>,

    /// TODO: docs
    runtime: MadRuntime,
}

impl Mad {
    /// Returns the dictionary describing the APIs exposed by the plugins that
    /// have been registered.
    pub fn api(self) -> Dictionary {
        let api = self.create_api();
        runtime::init(self.runtime);
        api
    }

    /// TODO: docs
    #[inline]
    fn create_api(&self) -> Dictionary {
        self.api
            .iter()
            .filter(|(_, api)| (!api.is_empty()))
            .map(|(name, api)| (*name, Object::from(api.clone())))
            .chain(core::iter::once((
                "setup",
                Function::from_fn(config::config).into(),
            )))
            .collect()
    }

    #[inline]
    pub fn new() -> Self {
        Self { api: HashMap::new(), runtime: MadRuntime::new() }
    }

    /// Registers a new plugin.
    #[inline]
    pub fn with_plugin<P: Plugin>(mut self, _plugin: P) -> Self {
        let plugin = P::init();
        self.api.insert(P::NAME, plugin.api());
        self.runtime.add_plugin(plugin);
        self
    }
}
