//! TODO: docs.

mod action;
mod action_ctx;
pub mod api;
mod async_ctx;
mod backend;
mod backend_handle;
mod byte_offset;
pub mod command;
mod constant;
pub mod executor;
mod function;
mod maybe_result;
pub mod module;
mod neovim_ctx;
pub mod notify;
mod plugin;
mod shared;
mod util;

pub use action::{Action, ActionCtx};
pub use async_ctx::AsyncCtx;
use backend::BackendExt;
pub use backend::{Backend, Key, MapAccess, Value};
use backend_handle::{BackendHandle, BackendMut};
pub use byte_offset::ByteOffset;
pub use constant::Constant;
pub use function::Function;
pub use maybe_result::MaybeResult;
pub use neovim_ctx::NeovimCtx;
pub use plugin::Plugin;
pub use shared::Shared;

/// TODO: docs.
pub type Name = &'static str;
