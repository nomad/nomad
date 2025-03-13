use core::any;

use serde::de::DeserializeOwned;

use crate::EditorCtx;
use crate::backend::Backend;
use crate::module::ApiCtx;
use crate::notify::Name;
use crate::plugin::PluginId;

/// TODO: docs.
pub trait Module<B: Backend>: 'static + Sized {
    /// TODO: docs.
    const NAME: Name;

    /// TODO: docs.
    type Config: DeserializeOwned;

    /// TODO: docs.
    fn api(&self, ctx: &mut ApiCtx<B>);

    /// TODO: docs.
    fn on_new_config(&self, new_config: Self::Config, ctx: &mut EditorCtx<B>);

    /// TODO: docs.
    #[allow(unused_variables)]
    fn on_init(&self, ctx: &mut EditorCtx<B>) {}

    #[inline]
    #[doc(hidden)]
    #[allow(private_interfaces)]
    fn id() -> ModuleId {
        ModuleId { type_id: any::TypeId::of::<Self>() }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ModuleId {
    type_id: any::TypeId,
}

impl From<PluginId> for ModuleId {
    #[inline]
    fn from(plugin_id: PluginId) -> Self {
        Self { type_id: plugin_id.type_id }
    }
}
