//! TODO: docs.

#![feature(min_specialization)]

pub mod action;
mod agent_id;
mod api;
mod backend;
mod base_backend;
mod buffer;
mod byte_offset;
pub mod command;
mod context;
mod cursor;
pub mod executor;
pub mod fs;
pub mod module;
pub mod notify;
pub mod plugin;
mod selection;
pub mod shared;
mod state;
mod util;

pub use agent_id::AgentId;
pub use api::{Api, ApiValue, Key, MapAccess, Value};
pub use backend::Backend;
pub use base_backend::BaseBackend;
pub use buffer::{Buffer, Chunks, Edit, Replacement};
pub use byte_offset::ByteOffset;
pub use context::{BorrowState, Borrowed, Context, NotBorrowed};
pub use cursor::Cursor;
pub use selection::Selection;
pub use shared::Shared;
