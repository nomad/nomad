//! TODO: docs.

#![feature(min_specialization)]

mod collab;
pub mod config;
mod convert;
mod editors;
mod event;
mod event_stream;
pub mod join;
pub mod leave;
mod list_ext;
pub mod project;
mod root_markers;
mod session;
pub mod start;
pub mod yank;

pub use collab::Collab;
#[cfg(feature = "mock")]
pub use editors::mock;
pub use editors::{CollabEditor, SessionId};
