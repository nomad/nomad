//! TODO: docs.

extern crate alloc;

mod byte_offset;
mod maybe_result;
mod point;
mod replacement;
mod shared;
mod text;

pub use byte_offset::ByteOffset;
pub use maybe_result::MaybeResult;
pub use nvim_oxi as oxi;
pub use point::Point;
pub use replacement::Replacement;
pub use shared::Shared;
pub use text::Text;
