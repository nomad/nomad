//! TODO: docs.

mod annotation;
pub mod binary;
pub mod fs;
mod project;
mod project_builder;
pub mod symlink;
pub mod text;

pub use collab_types::PeerId;
use collab_types::puff::abs_path;
#[cfg(feature = "serde")]
pub use project::DecodeError;
pub use project::{LocalPeerIsNotOwnerError, Project};
pub use project_builder::ProjectBuilder;
