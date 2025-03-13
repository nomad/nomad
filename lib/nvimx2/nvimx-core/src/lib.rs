//! TODO: docs.

#![feature(min_specialization)]
#![feature(precise_capturing_in_traits)]

pub mod action;
mod async_ctx;
pub mod backend;
mod buffer_ctx;
mod byte_offset;
pub mod command;
pub mod fs;
pub mod module;
mod editor_ctx;
pub mod notify;
pub mod plugin;
mod shared;
mod state;
mod util;

pub use async_ctx::AsyncCtx;
pub use buffer_ctx::BufferCtx;
pub use byte_offset::ByteOffset;
pub use editor_ctx::EditorCtx;
pub use shared::Shared;
