//! TODO: docs.

mod panic_infos;
mod plugin;

pub use panic_infos::{PanicInfo, PanicLocation};
pub use plugin::Plugin;
pub(crate) use plugin::{NO_COMMAND_NAME, PluginId};
