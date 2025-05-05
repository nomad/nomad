//! TODO: docs.

#![allow(missing_docs)]

pub mod api;
mod backend_ext;
pub mod buffer;
pub mod emitter;
pub mod executor;
pub mod fs;
mod mock;
pub mod serde;
pub mod value;

pub use backend_ext::BackendExt;
pub use mock::Mock;
pub use mock_macros::fs;
