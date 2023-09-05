use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;

use common::oxi::{self, Object};
use common::*;

use crate::config::ConfigError;

/// TODO: docs
pub(crate) trait ObjectSafePlugin {
    /// TODO: docs
    fn config(&mut self, global_enable: bool, config: Object);
}

impl<P: Plugin> ObjectSafePlugin for P {
    fn config(&mut self, global_enable: bool, config: Object) {
        let mut config =
            match serde_path_to_error::deserialize::<_, Enable<P::Config>>(
                oxi::serde::Deserializer::new(config),
            ) {
                Ok(config) => config,

                Err(err) => {
                    display_error(ConfigError::from(err), Some(P::NAME));
                    return;
                },
            };

        *config.enable_mut() &= global_enable;

        if let Err(err) = self.config(config) {
            display_error(err, Some(P::NAME));
        }
    }
}

/// TODO: docs
pub(crate) struct MadRuntime {
    plugins: HashMap<&'static str, Box<dyn ObjectSafePlugin>>,
}

impl MadRuntime {
    #[inline]
    pub(crate) fn add_plugin<P: Plugin>(&mut self, plugin: P) {
        let plugin = Box::new(plugin) as Box<dyn ObjectSafePlugin>;
        self.plugins.insert(P::NAME, plugin);
    }

    #[inline]
    pub(crate) fn get_plugin_mut(
        &mut self,
        name: &str,
    ) -> Option<&mut Box<dyn ObjectSafePlugin>> {
        self.plugins.get_mut(name)
    }

    #[inline]
    pub(crate) fn new() -> Self {
        Self { plugins: HashMap::new() }
    }
}

thread_local! {
    static MAD: OnceCell<RefCell<MadRuntime>> = OnceCell::new();
    static PLUGIN_NAMES: OnceCell<&'static [&'static str]> = OnceCell::new();
}

/// TODO: docs
pub(crate) fn with<F: FnOnce(&mut MadRuntime) -> R, R>(f: F) -> R {
    MAD.with(|mad| {
        let mad = mad.get().unwrap();
        let mad = &mut *mad.borrow_mut();
        f(mad)
    })
}

/// TODO: docs
pub(crate) fn init(rt: MadRuntime) {
    PLUGIN_NAMES.with(|names| {
        names
            .set(rt.plugins.keys().copied().collect::<Vec<_>>().leak())
            .unwrap();
    });

    MAD.with(|mad| {
        let _ = mad.set(RefCell::new(rt));
    });
}

/// TODO: docs
pub(crate) fn plugin_names() -> &'static [&'static str] {
    PLUGIN_NAMES.with(|names| *names.get().unwrap())
}
