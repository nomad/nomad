//! TODO: docs.

mod dir_entry;
mod fs;
mod fs_node;
mod fs_node_kind;
#[cfg(feature = "os-fs")]
pub mod os;
mod watcher;

pub use dir_entry::DirEntry;
#[doc(inline)]
pub use eerie::fs::{
    AbsPath,
    AbsPathBuf,
    AbsPathFromPathError,
    AbsPathNotAbsoluteError,
    AbsPathNotUtf8Error,
    FsNodeName,
    FsNodeNameBuf,
    InvalidFsNodeNameError,
};
pub use fs::Fs;
pub use fs_node::FsNode;
pub use fs_node_kind::FsNodeKind;
pub use watcher::{EventStream, FsEvent, Watcher};
