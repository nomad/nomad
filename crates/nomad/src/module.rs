use core::future::Future;

use serde::de::DeserializeOwned;

use crate::{Context, Editor, ModuleName};

/// TODO: docs.
pub trait Module<E: Editor>: 'static + Sized {
    /// TODO: docs.
    const NAME: ModuleName;

    /// TODO: docs.
    type Config: Default + DeserializeOwned;

    /// TODO: docs.
    fn init(ctx: &Context<E>) -> (Self, E::ModuleApi);

    /// TODO: docs.
    fn run(&mut self, ctx: &Context<E>) -> impl Future<Output = ()>;
}
