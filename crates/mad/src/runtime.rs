use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use common::nvim::{self, Object};
use common::*;

use crate::config::ConfigError;

/// TODO: docs
pub(crate) trait ConfigurablePlugin {
    /// TODO: docs
    fn config(&mut self, global_enable: bool, config: Object);
}

impl<P: Plugin> ConfigurablePlugin for P {
    fn config(&mut self, global_enable: bool, config: Object) {
        let mut config =
            match serde_path_to_error::deserialize::<_, Enable<P::Config>>(
                nvim::serde::Deserializer::new(config),
            ) {
                Ok(config) => config,

                Err(err) => {
                    display_error(ConfigError::from(err), Some(P::NAME));
                    return;
                },
            };

        *config.enable_mut() &= global_enable;

        self.update_config(config);
    }
}

/// TODO: docs
pub(crate) struct MadRuntime {
    plugins: HashMap<&'static str, Rc<dyn ConfigurablePlugin>>,
}

impl MadRuntime {
    pub(crate) fn add_plugin<P: Plugin>(&mut self, plugin: Rc<P>) {
        self.plugins.insert(P::NAME, plugin as _);
    }

    pub(crate) fn is_registered(&self, plugin: &str) -> bool {
        self.plugins.contains_key(plugin)
    }

    pub(crate) fn new() -> Self {
        Self { plugins: HashMap::new() }
    }

    pub(crate) fn update_config(
        &mut self,
        of_plugin: &str,
        global_enable: bool,
        config: Object,
    ) {
        if let Some(plugin) = self.plugins.get(of_plugin) {
            let plugin = Rc::clone(plugin);
            nvim::schedule(move |()| {
                // SAFETY: todo.
                let plugin = unsafe { crate::mad::rc_to_mut(&plugin) };
                plugin.config(global_enable, config);
                Ok(())
            });
        }
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
