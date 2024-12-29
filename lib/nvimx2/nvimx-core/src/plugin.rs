use core::marker::PhantomData;

use crate::module::Module;
use crate::{ActionName, Backend, BackendHandle};

/// TODO: docs.
pub trait Plugin<B: Backend>: Module<B, Namespace = Self> {
    /// TODO: docs.
    const COMMAND_NAME: &'static ActionName =
        ActionName::new(Self::NAME.uppercase_first().as_str());

    /// TODO: docs.
    const CONFIG_FN_NAME: &'static ActionName = ActionName::new("setup");

    /// TODO: docs.
    fn api(&self, ctx: PluginApiCtx<'_, Self, B>) -> B::Api<Self> {
        todo!();
    }
}

/// TODO: docs.
pub struct PluginApiCtx<'a, P: Plugin<B>, B: Backend> {
    api: B::Api<P>,
    backend: BackendHandle<B>,
    _phantom: PhantomData<&'a ()>,
}

impl<P, B> PluginApiCtx<'_, P, B>
where
    P: Plugin<B>,
    B: Backend,
{
    /// TODO: docs.
    #[inline]
    pub fn into_api(self) -> B::Api<P> {
        self.api
    }

    #[doc(hidden)]
    pub fn new(backend: B) -> Self {
        let backend = BackendHandle::new(backend);
        let api = backend.with_mut(|mut b| B::api::<P>(&mut b));
        Self { api, backend, _phantom: PhantomData }
    }
}
