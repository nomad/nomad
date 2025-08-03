//! TODO: docs.

#![allow(missing_docs)]
#![feature(async_fn_traits)]
#![feature(unboxed_closures)]

pub mod api;
pub mod buffer;
mod context_ext;
mod editor_ext;
pub mod emitter;
pub mod executor;
pub mod fs;
mod mock;
pub mod serde;
pub mod value;

/// This re-export is needed by the `mock::fs!` macro, but is not part of the
/// crate's public API.
#[doc(hidden)]
pub use abs_path::NodeName;
pub use context_ext::ContextExt;
pub use editor_ext::EditorExt;
pub use mock::Mock;
pub use mock_macros::fs;
