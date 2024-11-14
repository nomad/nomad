//! TODO: docs.

extern crate alloc;

pub mod autocmds;
mod command;
mod command_args;
pub mod config;
mod event;
pub mod events;
mod function;
mod log;
mod module;
mod module_api;
mod module_commands;
mod module_name;
mod nomad;
mod nomad_command;
mod serde;

pub use command::Command;
pub use command_args::CommandArgs;
pub use event::Event;
pub use function::Function;
pub use module::Module;
pub use module_api::ModuleApi;
pub use module_name::ModuleName;
pub use nomad::Nomad;
pub use nomad_macros::{action_name, module_name};
pub use nvim_oxi;
