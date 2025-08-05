//! Contains the message types related to file system operations.

use std::sync::Arc;

pub use puff::file::GlobalFileId;
pub use puff::ops::{
    DirectoryDeletion,
    DirectoryMove,
    FileDeletion,
    FileMove,
    Rename,
};

/// The message representing a directory creation.
pub type DirectoryCreation =
    puff::ops::DirectoryCreation<NewDirectoryContents>;

/// The message representing a file creation.
pub type FileCreation = puff::ops::FileCreation<NewFileContents>;

/// The contents of a newly created directory.
pub type NewDirectoryContents = ();

/// The contents of a newly created file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NewFileContents {
    /// The file contains arbitrary binary data.
    Binary(bytes::Bytes),

    /// The file is a symlink to the given target path.
    Symlink(Arc<str>),

    /// The file contains UTF-8 encoded text.
    Text(crop::Rope),
}
