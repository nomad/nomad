use core::future::Future;

use serde::de::DeserializeOwned;

use crate::config::ConfigReceiver;
use crate::ctx::NeovimCtx;
use crate::maybe_result::MaybeResult;
use crate::module_api::ModuleApi;
use crate::ModuleName;

/// TODO: docs.
pub trait Module: 'static + From<ConfigReceiver<Self>> {
    /// TODO: docs.
    const NAME: ModuleName;

    /// TODO: docs.
    type Config: Default + DeserializeOwned;

    /// TODO: docs.
    fn init(&self, ctx: NeovimCtx<'_>) -> ModuleApi<Self>;

    /// TODO: docs.
    fn run(
        self,
        ctx: NeovimCtx<'static>,
    ) -> impl Future<Output = impl MaybeResult<()>>;
}
