use serde::de::DeserializeOwned;

use crate::NeovimCtx;
use crate::backend::Backend;
use crate::module::ApiCtx;
use crate::notify::Name;

/// TODO: docs.
pub trait Module<B: Backend>: 'static + Sized {
    /// TODO: docs.
    const NAME: Name;

    /// TODO: docs.
    type Config: DeserializeOwned;

    /// TODO: docs.
    fn api(&self, ctx: &mut ApiCtx<Self, B>);

    /// TODO: docs.
    fn on_new_config(&self, new_config: Self::Config, ctx: &mut NeovimCtx<B>);
}
