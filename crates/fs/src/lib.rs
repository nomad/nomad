//! TODO: docs.

// Needed to bound the future returned by an `AsyncFnOnce` to `Send`.
#![cfg_attr(feature = "walk", feature(async_fn_traits))]
#![cfg_attr(feature = "walk", feature(unboxed_closures))]

mod directory;
mod file;
#[cfg(feature = "filter")]
pub mod filter;
mod fs;
mod metadata;
mod node;
mod node_kind;
#[cfg(feature = "os-fs")]
pub mod os;
mod symlink;
#[cfg(feature = "walk")]
pub mod walk;

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
pub use fs::{
    DeleteNodeError,
    Fs,
    GetDirError,
    MoveNodeError,
    ReadFileError,
    ReadFileToStringError,
};
pub use metadata::{Metadata, MetadataNameError};
pub use node::{Node, NodeDeleteError, NodeMoveError};
pub use node_kind::NodeKind;
pub use symlink::Symlink;
