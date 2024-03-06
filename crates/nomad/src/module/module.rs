use core::future::Future;

use serde::de::DeserializeOwned;

use crate::prelude::*;

/// TODO: docs
pub trait Module: 'static + Sized {
    /// TODO: docs
    const NAME: ModuleName;

    /// TODO: docs
    type Config: Default + DeserializeOwned;

    /// TODO: docs
    fn init(config: Get<Self::Config>, ctx: &InitCtx) -> Self;

    /// TODO: docs
    fn api(&self) -> Api;

    /// TODO: docs
    fn commands(&self) -> impl IntoIterator<Item = Command>;

    /// TODO: docs
    fn load(
        &self,
        // ctx: &mut SetCtx,
    ) -> impl Future<Output = impl MaybeResult<()>>;
}
