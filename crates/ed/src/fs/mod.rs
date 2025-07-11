//! TODO: docs.

mod directory;
mod file;
mod fs;
mod fs_event;
mod fs_node;
mod metadata;
mod node_kind;
#[cfg(feature = "os-fs")]
pub mod os;
mod symlink;

#[doc(inline)]
pub use abs_path::*;
pub use directory::{
    Directory,
    DirectoryEvent,
    NodeCreation,
    NodeDeletion,
    NodeMove,
    ReadNodeError,
    ReplicateError,
};
pub use file::{File, FileEvent, FileIdChange, FileModification};
pub use fs::{Fs, GetDirError, ReadFileError, ReadFileToStringError};
pub use fs_event::{FsEvent, FsEventKind};
pub use fs_node::{FsNode, NodeDeleteError};
pub use metadata::{Metadata, MetadataNameError};
pub use node_kind::NodeKind;
pub use symlink::Symlink;
