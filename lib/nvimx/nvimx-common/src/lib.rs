//! TODO: docs.

extern crate alloc;

mod apply;
mod byte_offset;
mod replacement;
mod shared;

pub use apply::Apply;
pub use byte_offset::ByteOffset;
pub use replacement::Replacement;
pub use shared::Shared;
