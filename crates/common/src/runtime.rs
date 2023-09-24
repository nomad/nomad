use std::any::Any;
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::nvim::{self, Object};
use crate::*;

/// TODO: docs
pub(crate) trait ConfigurablePlugin: Any {
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
                    display_error(err, Some(P::NAME));
                    return;
                },
            };

        *config.enable_mut() &= global_enable;

        self.update_config(config);
    }
}

/// TODO: docs
#[derive(Default)]
pub struct Runtime {
    plugins: HashMap<&'static str, Rc<RefCell<dyn ConfigurablePlugin>>>,
    plugin: HashMap<&'static str, Rc<RefCell<dyn Any>>>,
}

impl Runtime {
    #[track_caller]
    pub fn with_plugin<P, F, R>(&self, fun: F) -> R
    where
        P: Plugin,
        F: FnOnce(&P) -> R,
    {
        let Some(plugin) = self.plugin.get(P::NAME) else {
            panic!("Plugin not registered")
        };

        let plugin = plugin.borrow();

        let Some(plugin) = plugin.downcast_ref::<P>() else {
            panic!("Two or more plugins with the same name")
        };

        fun(plugin)
    }

    pub fn add_plugin<P: Plugin>(&mut self, plugin: Rc<RefCell<P>>) {
        self.plugins.insert(P::NAME, Rc::clone(&plugin) as _);
        self.plugin.insert(P::NAME, plugin as _);
    }

    pub fn is_registered(&self, plugin: &str) -> bool {
        self.plugins.contains_key(plugin)
    }

    pub fn update_config(
        &mut self,
        of_plugin: &str,
        global_enable: bool,
        config: Object,
    ) {
        if let Some(plugin) = self.plugins.get(of_plugin) {
            let plugin = Rc::clone(plugin);
            nvim::schedule(move |()| {
                // SAFETY: todo.
                let mut plugin = plugin.borrow_mut();
                plugin.config(global_enable, config);
                Ok(())
            });
        }
    }
}

thread_local! {
    static MAD: OnceCell<Rc<RefCell<Runtime>>> = OnceCell::new();
    static PLUGIN_NAMES: OnceCell<&'static [&'static str]> = OnceCell::new();
}

/// TODO: docs
pub fn with<F: FnOnce(&mut Runtime) -> R, R>(f: F) -> R {
    MAD.with(|mad| {
        let mad = mad.get().unwrap();
        let mut mad = mad.borrow_mut();
        f(&mut mad)
    })
}

/// TODO: docs
pub fn init(rt: Rc<RefCell<Runtime>>) {
    PLUGIN_NAMES.with(|names| {
        names
            .set(
                rt.borrow().plugins.keys().copied().collect::<Vec<_>>().leak(),
            )
            .unwrap();
    });

    MAD.with(|mad| {
        let _ = mad.set(rt);
    });
}

/// TODO: docs
pub fn plugin_names() -> &'static [&'static str] {
    PLUGIN_NAMES.with(|names| *names.get().unwrap())
}
