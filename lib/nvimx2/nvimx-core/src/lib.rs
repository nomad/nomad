//! TODO: docs.

#![feature(precise_capturing_in_traits)]

pub mod action;
mod async_ctx;
pub mod backend;
mod byte_offset;
pub mod command;
pub mod module;
mod neovim_ctx;
pub mod notify;
pub mod plugin;
mod shared;
mod state;
mod util;

pub use async_ctx::AsyncCtx;
pub use byte_offset::ByteOffset;
pub use neovim_ctx::NeovimCtx;
pub use shared::Shared;
