//! TODO: docs.

#![feature(precise_capturing_in_traits)]
#![allow(missing_docs)]

pub mod api;
mod backend;
pub mod buffer;
pub mod emitter;
pub mod executor;
pub mod fs;
pub mod serde;

pub use backend::TestBackend;
