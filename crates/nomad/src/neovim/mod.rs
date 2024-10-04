//! TODO: docs.

mod api;
mod buffer;
mod command;
mod config;
mod diagnostic;
mod executor;
mod function;
mod join_handle;
mod module_api;
mod neovim;
mod serde;
mod spawner;

pub use api::Api;
pub use buffer::{Buffer, BufferId, EditEvent};
pub use command::{
    command,
    Command,
    CommandArgs,
    CommandEvent,
    CommandHandle,
};
pub use config::ConfigEvent;
pub use diagnostic::{DiagnosticMessage, HighlightGroup};
pub use function::{function, Function, FunctionEvent, FunctionHandle};
pub use join_handle::NeovimJoinHandle;
pub use module_api::{module_api, ModuleApi};
pub use neovim::Neovim;
pub use spawner::NeovimSpawner;
