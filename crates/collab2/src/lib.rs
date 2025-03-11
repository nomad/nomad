//! TODO: docs.

#![feature(min_specialization)]
#![feature(precise_capturing_in_traits)]

pub mod backend;
mod collab;
mod config;
pub mod join;
mod leave;
mod project;
mod root_markers;
mod session;
pub mod start;
mod yank;

pub use backend::CollabBackend;
pub use collab::Collab;
pub use leave::Leave;
pub use project::Project;
pub use yank::Yank;
