//! TODO: docs.

mod action;
pub mod api;
mod async_ctx;
mod backend;
mod backend_handle;
mod byte_offset;
pub mod command;
pub mod executor;
mod function;
mod maybe_result;
pub mod module;
mod neovim_ctx;
pub mod notify;
mod plugin;
mod shared;

pub use action::{Action, ActionName};
pub use async_ctx::AsyncCtx;
pub use backend::Backend;
use backend::BackendExt;
use backend_handle::{BackendHandle, BackendMut};
pub use byte_offset::ByteOffset;
pub use function::Function;
pub use maybe_result::MaybeResult;
pub use neovim_ctx::NeovimCtx;
pub use plugin::{Plugin, PluginApiCtx};
pub use shared::Shared;
