use crate::backend::Backend;
use crate::module::{self, Module};
use crate::notify::Name;
use crate::state::StateHandle;

pub(crate) const NO_COMMAND_NAME: &str = "�";

/// TODO: docs.
pub trait Plugin<B: Backend>: Module<B> {
    /// TODO: docs.
    const COMMAND_NAME: Name = NO_COMMAND_NAME;

    /// TODO: docs.
    const CONFIG_FN_NAME: Name = "setup";

    /// TODO: docs.
    fn panic_handler(&self) -> Option<Box<dyn FnMut() + 'static>> {
        todo!()
    }

    /// TODO: docs.
    fn tracing_subscriber(&self) -> Option<Box<dyn FnMut() + 'static>> {
        todo!()
    }

    #[doc(hidden)]
    #[track_caller]
    fn api(self, backend: B) -> B::Api {
        StateHandle::new(backend).with_mut(|s| module::build_api(self, s))
    }
}
