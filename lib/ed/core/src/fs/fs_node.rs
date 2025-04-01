use core::error::Error;
use core::fmt;

use abs_path::AbsPath;

use crate::fs::{self, Directory, File, NodeKind, Symlink};

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
pub enum NodeDeleteError<Fs: fs::Fs> {
    /// TODO: docs.
    File(<Fs::File as File>::DeleteError),

    /// TODO: docs.
    Directory(<Fs::Directory as Directory>::DeleteError),

    /// TODO: docs.
    Symlink(<Fs::Symlink as Symlink>::DeleteError),
}

/// TODO: docs.
#[derive(derive_more::Debug)]
#[debug(bound(Fs: fs::Fs))]
pub enum NodeMetadataError<Fs: fs::Fs> {
    /// TODO: docs.
    File(<Fs::File as File>::MetadataError),

    /// TODO: docs.
    Directory(<Fs::Directory as Directory>::MetadataError),

    /// TODO: docs.
    Symlink(<Fs::Symlink as Symlink>::MetadataError),
}

impl<Fs: fs::Fs> FsNode<Fs> {
    /// TODO: docs.
    #[inline]
    pub async fn delete(self) -> Result<(), NodeDeleteError<Fs>> {
        match self {
            Self::File(file) => {
                file.delete().await.map_err(NodeDeleteError::File)
            },
            Self::Directory(dir) => {
                dir.delete().await.map_err(NodeDeleteError::Directory)
            },
            Self::Symlink(symlink) => {
                symlink.delete().await.map_err(NodeDeleteError::Symlink)
            },
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn id(&self) -> Fs::NodeId {
        match self {
            Self::File(file) => file.id(),
            Self::Directory(dir) => dir.id(),
            Self::Symlink(symlink) => symlink.id(),
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
    pub fn kind(&self) -> NodeKind {
        match self {
            Self::File(_) => NodeKind::File,
            Self::Directory(_) => NodeKind::Directory,
            Self::Symlink(_) => NodeKind::Symlink,
        }
    }

    /// TODO: docs.
    #[inline]
    pub async fn meta(&self) -> Result<Fs::Metadata, NodeMetadataError<Fs>> {
        match self {
            Self::File(file) => {
                file.meta().await.map_err(NodeMetadataError::File)
            },
            Self::Directory(dir) => {
                dir.meta().await.map_err(NodeMetadataError::Directory)
            },
            Self::Symlink(symlink) => {
                symlink.meta().await.map_err(NodeMetadataError::Symlink)
            },
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn path(&self) -> &AbsPath {
        match self {
            Self::File(file) => file.path(),
            Self::Directory(directory) => directory.path(),
            Self::Symlink(symlink) => symlink.path(),
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

impl<Fs: fs::Fs> PartialEq for NodeDeleteError<Fs>
where
    <Fs::File as File>::DeleteError: PartialEq,
    <Fs::Directory as Directory>::DeleteError: PartialEq,
    <Fs::Symlink as Symlink>::DeleteError: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        use NodeDeleteError::*;

        match (self, other) {
            (File(l), File(r)) => l == r,
            (Directory(l), Directory(r)) => l == r,
            (Symlink(l), Symlink(r)) => l == r,
            _ => false,
        }
    }
}

impl<Fs: fs::Fs> fmt::Display for NodeDeleteError<Fs> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(err) => fmt::Display::fmt(err, f),
            Self::Directory(err) => fmt::Display::fmt(err, f),
            Self::Symlink(err) => fmt::Display::fmt(err, f),
        }
    }
}

impl<Fs: fs::Fs> Error for NodeDeleteError<Fs> {}

impl<Fs: fs::Fs> PartialEq for NodeMetadataError<Fs>
where
    <Fs::File as File>::MetadataError: PartialEq,
    <Fs::Directory as Directory>::MetadataError: PartialEq,
    <Fs::Symlink as Symlink>::MetadataError: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        use NodeMetadataError::*;

        match (self, other) {
            (File(l), File(r)) => l == r,
            (Directory(l), Directory(r)) => l == r,
            (Symlink(l), Symlink(r)) => l == r,
            _ => false,
        }
    }
}

impl<Fs: fs::Fs> fmt::Display for NodeMetadataError<Fs> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(err) => fmt::Display::fmt(err, f),
            Self::Directory(err) => fmt::Display::fmt(err, f),
            Self::Symlink(err) => fmt::Display::fmt(err, f),
        }
    }
}

impl<Fs: fs::Fs> Error for NodeMetadataError<Fs> {}
