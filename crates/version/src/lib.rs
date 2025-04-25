//! TODO: docs.

mod emit_version;
mod version;

mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}

pub use emit_version::EmitVersion;
pub use version::{VERSION, Version};
