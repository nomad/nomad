//! TODO: docs.

mod action;
mod async_ctx;
mod backend;
mod command;
mod command_args;
pub mod executor;
mod function;
mod maybe_result;
mod module;
mod module_api;
mod neovim_ctx;
mod plugin;
mod plugin_api;
mod shared;

pub use action::{Action, ActionName};
pub use async_ctx::AsyncCtx;
pub use backend::Backend;
pub use command::Command;
pub use command_args::CommandArgs;
pub use function::Function;
pub use maybe_result::MaybeResult;
pub use module::{Module, ModuleCtx, ModuleName};
pub use module_api::ModuleApi;
pub use neovim_ctx::NeovimCtx;
pub use plugin::{Plugin, PluginCtx, PluginName};
pub use plugin_api::PluginApi;
pub use shared::Shared;
