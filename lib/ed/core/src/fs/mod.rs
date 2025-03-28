//! TODO: docs.

mod directory;
mod file;
mod fs;
mod fs_event;
mod fs_node;
mod fs_node_kind;
mod metadata;
#[cfg(feature = "os-fs")]
pub mod os;
mod symlink;

#[doc(inline)]
pub use abs_path::*;
pub use directory::{
    Directory,
    NodeDeletion,
    DirectoryEvent,
    NodeMove,
    NodeCreation,
};
pub use file::{File, FileEvent};
pub use fs::Fs;
pub use fs_event::{FsEvent, FsEventKind};
pub use fs_node::{FsNode, NodeDeleteError, NodeMetadataError};
pub use fs_node_kind::FsNodeKind;
pub use metadata::{Metadata, MetadataNameError};
pub use symlink::Symlink;
