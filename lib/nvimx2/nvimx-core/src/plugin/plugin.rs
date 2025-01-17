use core::any::Any;

use crate::backend::Backend;
use crate::module::{self, Module};
use crate::notify::Name;
use crate::state::StateHandle;
use crate::{NeovimCtx, notify};

pub(crate) const NO_COMMAND_NAME: &str = "�";

/// TODO: docs.
pub trait Plugin<B: Backend>: Module<B> {
    /// TODO: docs.
    const COMMAND_NAME: Name = NO_COMMAND_NAME;

    /// TODO: docs.
    const CONFIG_FN_NAME: Name = "setup";

    #[doc(hidden)]
    #[track_caller]
    fn api(self, backend: B) -> B::Api {
        StateHandle::new(backend).with_mut(|s| module::build_api(self, s))
    }

    /// TODO: docs.
    fn handle_panic(
        &self,
        panic_payload: Box<dyn Any + Send + 'static>,
        ctx: &mut NeovimCtx<B>,
    ) {
        let mut message = notify::Message::from_str("panicked");

        let maybe_payload_str = panic_payload
            .downcast_ref::<String>()
            .map(|s| &**s)
            .or_else(|| panic_payload.downcast_ref::<&str>().copied());

        if let Some(payload) = maybe_payload_str {
            message.push_str(": ").push_info(payload);
        }

        ctx.emit_error(message);
    }

    /// TODO: docs.
    fn tracing_subscriber(&self) -> Option<Box<dyn FnMut() + 'static>> {
        todo!()
    }
}
