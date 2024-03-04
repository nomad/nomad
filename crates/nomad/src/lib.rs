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
mod enable;
pub mod log;
mod maybe_result;
mod module;
mod module_name;
mod nomad;
pub mod runtime;

pub use action::Action;
pub use action_name::ActionName;
pub use api::Api;
pub use command::Command;
pub use enable::{DefaultEnable, EnableConfig};
pub use macros::{action_name, module_name};
pub use maybe_result::MaybeResult;
pub use module::Module;
pub use module_name::ModuleName;
pub use nomad::Nomad;
pub use nvim_oxi as nvim;

pub mod prelude {
    //! TODO: docs

    pub use ctx::*;
    pub use log::*;
    pub use runtime::*;

    pub use super::*;
}
