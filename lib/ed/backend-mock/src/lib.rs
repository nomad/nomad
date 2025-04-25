//! TODO: docs.

#![allow(missing_docs)]

pub mod api;
mod backend;
mod backend_ext;
pub mod buffer;
pub mod emitter;
pub mod executor;
pub mod fs;
pub mod serde;
pub mod value;

pub use backend::Mock;
pub use backend_ext::BackendExt;
pub use mock_macros::fs;
