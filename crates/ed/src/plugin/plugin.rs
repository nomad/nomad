use core::any;

use smol_str::ToSmolStr;

use crate::backend::Backend;
use crate::module::{self, Module};
use crate::notify::{self, Name};
use crate::plugin::PanicInfo;
use crate::state::StateHandle;
use crate::{Borrowed, Context};

pub(crate) const NO_COMMAND_NAME: &str = "�";

/// TODO: docs.
pub trait Plugin<B: Backend>: Module<B> {
    /// TODO: docs.
    const COMMAND_NAME: Name = NO_COMMAND_NAME;

    /// TODO: docs.
    const CONFIG_FN_NAME: Name = "setup";

    /// TODO: docs.
    fn handle_panic(
        &self,
        panic_info: PanicInfo,
        ctx: &mut Context<B, Borrowed<'_>>,
    ) {
        let mut message = notify::Message::from_str("panicked");

        if let Some(location) = &panic_info.location {
            message.push_str(" at ").push_info(location.to_smolstr());
        }
        if let Some(payload) = panic_info.payload_as_str() {
            message.push_str(": ").push_info(payload);
        }

        ctx.emit_error(message);
    }

    #[doc(hidden)]
    #[track_caller]
    fn api(self, backend: B) -> B::Api {
        StateHandle::new(backend).with_mut(|s| module::build_api(self, s))
    }

    #[inline]
    #[doc(hidden)]
    #[allow(private_interfaces)]
    fn id() -> PluginId {
        PluginId { type_id: any::TypeId::of::<Self>() }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PluginId {
    pub(crate) type_id: any::TypeId,
}
