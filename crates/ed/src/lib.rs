//! TODO: docs.

#![feature(min_specialization)]

pub mod action;
pub mod backend;
mod byte_offset;
pub mod command;
mod context;
pub mod executor;
pub mod fs;
pub mod module;
pub mod notify;
pub mod plugin;
pub mod shared;
mod state;
mod util;

pub use backend::Backend;
pub use byte_offset::ByteOffset;
pub use context::{BorrowState, Borrowed, Context, NotBorrowed};
pub use shared::Shared;
