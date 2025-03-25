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
    ChildCreation,
    Directory,
    DirectoryDeletion,
    DirectoryEvent,
    DirectoryMove,
};
pub use file::File;
pub use fs::Fs;
pub use fs_event::{FsEvent, FsEventKind};
pub use fs_node::{DeleteNodeError, FsNode};
pub use fs_node_kind::FsNodeKind;
pub use metadata::Metadata;
pub use symlink::Symlink;
