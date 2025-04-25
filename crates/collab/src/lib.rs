//! TODO: docs.

#![feature(min_specialization)]

mod backend;
mod collab;
pub mod config;
mod event;
pub mod join;
pub mod leave;
pub mod project;
mod root_markers;
mod seq_ext;
mod session;
pub mod start;
pub mod yank;

pub use backend::CollabBackend;
#[cfg(feature = "mock")]
pub use backend::mock;
pub use collab::Collab;
