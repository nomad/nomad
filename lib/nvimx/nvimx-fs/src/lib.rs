//! TODO: docs.

extern crate alloc;

mod dir_entry;
mod fs;
mod fs_node;
mod fs_node_kind;
pub mod os_fs;

pub use dir_entry::DirEntry;
pub use e31e::fs::{
    AbsPath,
    AbsPathBuf,
    FsNodeName,
    FsNodeNameBuf,
    InvalidFsNodeNameError,
};
pub use fs::Fs;
pub use fs_node::FsNode;
pub use fs_node_kind::FsNodeKind;
