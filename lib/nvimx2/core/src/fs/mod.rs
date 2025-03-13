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

pub use directory::Directory;
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
pub use file::File;
pub use fs::Fs;
pub use fs_event::{FsEvent, FsEventKind};
pub use fs_node::{DeleteNodeError, FsNode};
pub use fs_node_kind::FsNodeKind;
pub use metadata::Metadata;
pub use symlink::Symlink;
