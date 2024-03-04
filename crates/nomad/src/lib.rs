//! # Nomad
//!
//! TODO: docs

extern crate alloc;

mod action;
mod action_name;
mod api;
mod command;
mod config;
pub mod ctx;
pub mod log;
mod maybe_result;
pub mod module;
mod nomad;
pub mod runtime;

pub use action::Action;
pub use action_name::ActionName;
pub use api::Api;
pub use command::Command;
pub use macros::action_name;
pub use maybe_result::MaybeResult;
pub use nomad::Nomad;
pub use nvim_oxi as nvim;

pub mod prelude {
    //! TODO: docs

    pub use config::EnableConfig;
    pub use ctx::*;
    pub use log::*;
    pub use module::*;
    pub use runtime::*;

    pub use super::*;
}
