use crate::api::{Api, ModuleApi};
use crate::module::{ApiCtx, CommandBuilder, Module};
use crate::{ActionName, Backend, BackendHandle};

/// TODO: docs.
pub trait Plugin<B: Backend>: Module<B> {
    /// TODO: docs.
    const COMMAND_NAME: &'static ActionName =
        ActionName::new(Self::NAME.uppercase_first().as_str());

    /// TODO: docs.
    const CONFIG_FN_NAME: &'static ActionName = ActionName::new("setup");

    #[doc(hidden)]
    fn api(&self, mut backend: B) -> B::Api<Self> {
        let mut api = B::api::<Self>(&mut backend);
        let backend = BackendHandle::new(backend);
        let mut module_api = api.as_module();
        let mut command_builder = CommandBuilder::new::<Self>();
        let api_ctx = ApiCtx::<Self, _, _>::new(
            &mut module_api,
            &mut command_builder,
            &backend,
        );
        Module::api(self, api_ctx);
        module_api.finish();
        api
    }
}
