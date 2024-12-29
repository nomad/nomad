use crate::module::{ApiCtx, Module};
use crate::{ActionName, Backend, BackendHandle};

/// TODO: docs.
pub trait Plugin<B: Backend>: Module<B, Namespace = Self> {
    /// TODO: docs.
    const COMMAND_NAME: &'static ActionName =
        ActionName::new(Self::NAME.uppercase_first().as_str());

    /// TODO: docs.
    const CONFIG_FN_NAME: &'static ActionName = ActionName::new("setup");

    #[doc(hidden)]
    fn api(&self, mut backend: B) -> B::Api<Self> {
        let mut api = B::api::<Self>(&mut backend);
        let backend = BackendHandle::new(backend);
        let api_ctx = ApiCtx::<Self, _>::new(&mut api, &backend);
        Module::api(self, api_ctx);
        api
    }
}
