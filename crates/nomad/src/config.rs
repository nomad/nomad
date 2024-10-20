//! TODO: docs.

use fxhash::FxHashMap;
use nvim_oxi::Object as NvimObject;

use crate::{Module, ModuleName};

/// TODO: docs.
pub struct ConfigReceiver<M: Module> {
    _config: M::Config,
}

/// TODO: docs.
#[derive(Default)]
pub(crate) struct Setup {
    /// Map from [`ModuleName`] to the [`ConfigReceiver`] for that module.]
    config_senders: FxHashMap<ModuleName, ConfigSender>,

    /// The keys of the `config_senders` map, ordered alphabetically.
    module_names: Vec<ModuleName>,
}

/// TODO: docs.
struct ConfigSender {}

impl<M: Module> ConfigReceiver<M> {
    pub async fn recv(&mut self) -> M::Config {
        todo!();
    }
}

impl Setup {
    pub(crate) const NAME: &'static str = "setup";

    /// Adds a module to the setup function.
    ///
    /// # Panics
    ///
    /// Panics if the module's name is `"setup"` or equal to the name of a
    /// previously added module.
    #[track_caller]
    pub(crate) fn add_module<M: Module>(&mut self) -> ConfigReceiver<M> {
        todo!();
    }

    pub(crate) fn into_fn(self) -> impl Fn(NvimObject) + 'static {
        |_obj| todo!()
    }
}

impl ConfigSender {
    fn send(&self, _config: NvimObject) {
        todo!();
    }
}
