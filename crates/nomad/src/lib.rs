//! # Nomad
//!
//! TODO: docs

pub use macros;
#[doc(hidden)]
pub use nvim;

pub mod action;
pub mod api;
mod apply;
mod autocmd_id;
mod buffer;
mod buffer_id;
mod buffer_snapshot;
mod byte_offset;
mod command;
mod command_args;
mod config;
mod crdt_replacement;
mod ctx;
mod edit;
mod editor_ctx;
mod editor_id;
mod from_with;
pub mod log;
pub mod maybe_future;
pub mod maybe_result;
pub mod module;
mod nomad;
mod nvim_buffer;
mod point;
mod replacement;
pub mod runtime;
mod serde;
mod shared;
pub mod streams;
#[cfg(feature = "tests")]
pub mod tests;
mod utils;
pub mod warning;

pub mod prelude {
    //! TODO: docs

    pub use macros::*;
    #[doc(hidden)]
    pub use nvim;
    pub use ui::*;

    pub use crate::action::*;
    pub use crate::api::*;
    pub use crate::apply::Apply;
    pub use crate::buffer::Buffer;
    pub use crate::buffer_id::BufferId;
    pub use crate::buffer_snapshot::BufferSnapshot;
    pub use crate::byte_offset::ByteOffset;
    pub use crate::command_args::*;
    pub use crate::crdt_replacement::CrdtReplacement;
    pub use crate::ctx::Ctx;
    pub use crate::edit::Edit;
    pub use crate::editor_ctx::EditorCtx;
    pub use crate::editor_id::EditorId;
    pub use crate::from_with::{FromWith, IntoWith};
    pub use crate::log::*;
    pub use crate::maybe_future::*;
    pub use crate::maybe_result::*;
    pub use crate::module::*;
    pub use crate::nvim_buffer::{NvimBuffer, NvimBufferDoesntExistError};
    pub use crate::point::Point;
    pub use crate::replacement::Replacement;
    pub use crate::runtime::*;
    pub use crate::shared::Shared;
    pub use crate::streams::*;
    pub use crate::warning::*;
    pub use crate::Nomad;
}

pub(crate) use autocmd_id::AutocmdId;
pub(crate) use command::{Command, ModuleCommands};
pub(crate) use config::Config;
#[cfg(feature = "tests")]
pub use macros::test;
pub(crate) use prelude::*;
pub(crate) use serde::{deserialize, serialize, DeserializeError};

pub use crate::nomad::Nomad;
