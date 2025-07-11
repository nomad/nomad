use abs_path::AbsPath;

use crate::fs::{self, Directory, File, NodeKind, Symlink};

/// TODO: docs.
#[derive(cauchy::Debug, cauchy::PartialEq)]
pub enum FsNode<Fs: fs::Fs> {
    /// TODO: docs.
    File(Fs::File),

    /// TODO: docs.
    Directory(Fs::Directory),

    /// TODO: docs.
    Symlink(Fs::Symlink),
}

/// TODO: docs.
#[derive(
    cauchy::Debug, derive_more::Display, cauchy::Error, cauchy::PartialEq,
)]
#[display("{_0}")]
pub enum NodeDeleteError<Fs: fs::Fs> {
    /// TODO: docs.
    File(<Fs::File as File>::DeleteError),

    /// TODO: docs.
    Directory(<Fs::Directory as Directory>::DeleteError),

    /// TODO: docs.
    Symlink(<Fs::Symlink as Symlink>::DeleteError),
}

/// TODO: docs.
#[derive(
    cauchy::Debug, derive_more::Display, cauchy::Error, cauchy::PartialEq,
)]
#[display("{_0}")]
pub enum NodeMoveError<Fs: fs::Fs> {
    /// TODO: docs.
    File(<Fs::File as File>::MoveError),

    /// TODO: docs.
    Directory(<Fs::Directory as Directory>::MoveError),

    /// TODO: docs.
    Symlink(<Fs::Symlink as Symlink>::MoveError),
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
    pub fn meta(&self) -> Fs::Metadata {
        match self {
            Self::File(file) => file.meta(),
            Self::Directory(dir) => dir.meta(),
            Self::Symlink(symlink) => symlink.meta(),
        }
    }

    /// TODO: docs.
    #[inline]
    pub async fn r#move(
        &self,
        new_path: &AbsPath,
    ) -> Result<(), NodeMoveError<Fs>> {
        match self {
            Self::File(file) => {
                file.r#move(new_path).await.map_err(NodeMoveError::File)
            },
            Self::Directory(dir) => {
                dir.r#move(new_path).await.map_err(NodeMoveError::Directory)
            },
            Self::Symlink(symlink) => {
                symlink.r#move(new_path).await.map_err(NodeMoveError::Symlink)
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

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn unwrap_directory(self) -> Fs::Directory {
        match self {
            Self::Directory(dir) => dir,
            other => panic!("expected directory, got {:?}", other.kind()),
        }
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn unwrap_file(self) -> Fs::File {
        match self {
            Self::File(file) => file,
            other => panic!("expected file, got {:?}", other.kind()),
        }
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn unwrap_symlink(self) -> Fs::Symlink {
        match self {
            Self::Symlink(symlink) => symlink,
            other => panic!("expected symlink, got {:?}", other.kind()),
        }
    }
}
