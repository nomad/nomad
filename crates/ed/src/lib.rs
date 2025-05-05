//! TODO: docs.

#![feature(min_specialization)]

pub mod action;
mod async_ctx;
pub mod backend;
mod byte_offset;
pub mod command;
mod editor_ctx;
pub mod fs;
pub mod module;
pub mod notify;
pub mod plugin;
pub mod shared;
mod state;
mod util;

pub use async_ctx::AsyncCtx;
pub use byte_offset::ByteOffset;
pub use editor_ctx::EditorCtx;
pub use shared::Shared;
