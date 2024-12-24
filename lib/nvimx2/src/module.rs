use serde::de::DeserializeOwned;

use crate::Backend;

/// TODO: docs.
pub trait Module<B: Backend>: 'static {
    /// TODO: docs.
    const NAME: ModuleName;

    /// TODO: docs.
    type Plugin: Plugin<B>;

    /// TODO: docs.
    type Config: DeserializeOwned;

    /// TODO: docs.
    type Docs;

    /// TODO: docs.
    fn module_api(&self, ctx: ModuleCtx<'_, B>) -> ModuleApi<Self, B>;

    /// TODO: docs.
    fn on_config_changed(&mut self, new_config: Self::Config);

    /// TODO: docs.
    fn docs() -> Self::Docs;
}
