use core::future::Future;

use serde::de::DeserializeOwned;

use crate::{Context, Editor, ModuleName};

/// TODO: docs.
pub trait Module<E: Editor>: Sized {
    /// TODO: docs.
    const NAME: ModuleName;

    /// TODO: docs.
    type Config: Default + DeserializeOwned;

    /// TODO: docs.
    fn api(ctx: &Context<E>) -> E::ModuleApi<Self>;

    /// TODO: docs.
    fn run(&mut self, ctx: &Context<E>) -> impl Future<Output = ()>;
}
