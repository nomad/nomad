/// TODO: docs.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FsNodeKind {
    /// TODO: docs.
    File,

    /// TODO: docs.
    Directory,

    /// TODO: docs.
    Symlink,
}

impl FsNodeKind {
    /// TODO: docs.
    pub fn is_dir(self) -> bool {
        matches!(self, Self::Directory)
    }

    /// TODO: docs.
    pub fn is_file(self) -> bool {
        matches!(self, Self::File)
    }

    /// TODO: docs.
    pub fn is_symlink(self) -> bool {
        matches!(self, Self::Symlink)
    }
}

impl TryFrom<std::fs::FileType> for FsNodeKind {
    type Error = std::fs::FileType;

    fn try_from(file_type: std::fs::FileType) -> Result<Self, Self::Error> {
        Ok(if file_type.is_file() {
            Self::File
        } else if file_type.is_dir() {
            Self::Directory
        } else if file_type.is_symlink() {
            Self::Symlink
        } else {
            return Err(file_type);
        })
    }
}
