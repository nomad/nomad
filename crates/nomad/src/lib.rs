//! # Nomad
//!
//! TODO: docs

extern crate alloc;

pub mod action;
pub mod api;
mod command;
mod config;
pub mod log;
pub mod maybe_future;
pub mod maybe_result;
pub mod module;
mod nomad;
pub mod runtime;
mod serde;
pub mod warning;

pub use nomad::Nomad;

pub mod prelude {
    //! TODO: docs

    pub use macros::Ready;
    pub use nvim;

    pub use crate::action::*;
    pub use crate::api::*;
    pub use crate::command::*;
    pub use crate::log::*;
    pub use crate::maybe_future::*;
    pub use crate::maybe_result::*;
    pub use crate::module::*;
    pub use crate::runtime::*;
    pub use crate::warning::*;
    pub use crate::Nomad;
}
