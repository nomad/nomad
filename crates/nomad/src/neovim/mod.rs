//! TODO: docs.

mod api;
mod autocmd;
mod buffer;
mod command;
mod config;
pub mod events;
mod executor;
mod function;
mod join_handle;
mod module_api;
mod neovim;
mod offset;
mod point;
mod serde;
mod spawner;

pub use api::Api;
pub use autocmd::{Autocmd, AutocmdId, ShouldDetach};
pub use buffer::{Buffer, BufferId};
pub use command::{command, Command, CommandHandle};
pub use function::{function, Function, FunctionHandle};
pub use join_handle::NeovimJoinHandle;
pub use module_api::{module_api, ModuleApi};
pub use neovim::Neovim;
use offset::Offset;
pub use point::Point;
pub use spawner::NeovimSpawner;
