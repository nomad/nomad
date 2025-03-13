use core::error::Error;
use core::fmt;

use crate::fs::{self, Directory, File, FsNodeKind, Symlink};

/// TODO: docs.
pub enum FsNode<Fs: fs::Fs> {
    /// TODO: docs.
    File(Fs::File),

    /// TODO: docs.
    Directory(Fs::Directory),

    /// TODO: docs.
    Symlink(Fs::Symlink),
}

/// TODO: docs.
#[derive(derive_more::Debug)]
#[debug(bound(Fs: fs::Fs))]
pub enum DeleteNodeError<Fs: fs::Fs> {
    /// TODO: docs.
    File(<Fs::File as File>::DeleteError),

    /// TODO: docs.
    Directory(<Fs::Directory as Directory>::DeleteError),

    /// TODO: docs.
    Symlink(<Fs::Symlink as Symlink>::DeleteError),
}

impl<Fs: fs::Fs> FsNode<Fs> {
    /// TODO: docs.
    #[inline]
    pub async fn delete(self) -> Result<(), DeleteNodeError<Fs>> {
        match self {
            Self::File(file) => {
                file.delete().await.map_err(DeleteNodeError::File)
            },
            Self::Directory(dir) => {
                dir.delete().await.map_err(DeleteNodeError::Directory)
            },
            Self::Symlink(symlink) => {
                symlink.delete().await.map_err(DeleteNodeError::Symlink)
            },
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.kind().is_dir()
    }

    /// TODO: docs.
    #[inline]
    pub fn is_file(&self) -> bool {
        self.kind().is_file()
    }

    /// TODO: docs.
    #[inline]
    pub fn kind(&self) -> FsNodeKind {
        match self {
            Self::File(_) => FsNodeKind::File,
            Self::Directory(_) => FsNodeKind::Directory,
            Self::Symlink(_) => FsNodeKind::Symlink,
        }
    }
}

impl<Fs: fs::Fs> fmt::Debug for FsNode<Fs>
where
    Fs::File: fmt::Debug,
    Fs::Directory: fmt::Debug,
    Fs::Symlink: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(file) => fmt::Debug::fmt(file, f),
            Self::Directory(dir) => fmt::Debug::fmt(dir, f),
            Self::Symlink(symlink) => fmt::Debug::fmt(symlink, f),
        }
    }
}

impl<Fs: fs::Fs> PartialEq for FsNode<Fs>
where
    Fs::File: PartialEq,
    Fs::Directory: PartialEq,
    Fs::Symlink: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        use FsNode::*;

        match (self, other) {
            (File(l), File(r)) => l == r,
            (Directory(l), Directory(r)) => l == r,
            (Symlink(l), Symlink(r)) => l == r,
            _ => false,
        }
    }
}

impl<Fs: fs::Fs> PartialEq for DeleteNodeError<Fs>
where
    <Fs::File as File>::DeleteError: PartialEq,
    <Fs::Directory as Directory>::DeleteError: PartialEq,
    <Fs::Symlink as Symlink>::DeleteError: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        use DeleteNodeError::*;

        match (self, other) {
            (File(l), File(r)) => l == r,
            (Directory(l), Directory(r)) => l == r,
            (Symlink(l), Symlink(r)) => l == r,
            _ => false,
        }
    }
}

impl<Fs: fs::Fs> fmt::Display for DeleteNodeError<Fs> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(err) => fmt::Display::fmt(err, f),
            Self::Directory(err) => fmt::Display::fmt(err, f),
            Self::Symlink(err) => fmt::Display::fmt(err, f),
        }
    }
}

impl<Fs: fs::Fs> Error for DeleteNodeError<Fs> {}
