use core::error::Error;
use core::fmt::Debug;
use core::future::Future;
use core::hash::Hash;

use abs_path::AbsPathBuf;

use crate::fs::{AbsPath, Directory, File, FsNode, Metadata, Symlink};

/// TODO: docs.
pub trait Fs: Clone + Send + Sync + 'static {
    /// TODO: docs.
    type Directory: Directory<Fs = Self>;

    /// TODO: docs.
    type File: File<Fs = Self>;

    /// TODO: docs.
    type Symlink: Symlink<Fs = Self>;

    /// TODO: docs.
    type Metadata: Metadata<Fs = Self>;

    /// TODO: docs.
    type NodeId: Debug + Clone + Eq + Hash + Send + Sync;

    /// TODO: docs.
    type Timestamp: Clone + Ord;

    /// TODO: docs.
    type CreateDirectoriesError: Error + Send;

    /// TODO: docs.
    type NodeAtPathError: Error + Send;

    /// TODO: docs.
    fn create_all_missing_directories<P: AsRef<AbsPath> + Send>(
        &self,
        path: P,
    ) -> impl Future<
        Output = Result<Self::Directory, Self::CreateDirectoriesError>,
    > + Send;

    /// TODO: docs.
    fn node_at_path<P: AsRef<AbsPath> + Send>(
        &self,
        path: P,
    ) -> impl Future<
        Output = Result<Option<FsNode<Self>>, Self::NodeAtPathError>,
    > + Send;

    /// TODO: docs.
    fn now(&self) -> Self::Timestamp;

    /// TODO: docs.
    fn exists<P: AsRef<AbsPath> + Send>(
        &self,
        path: P,
    ) -> impl Future<Output = Result<bool, Self::NodeAtPathError>> {
        async move { self.node_at_path(path).await.map(|opt| opt.is_some()) }
    }

    /// TODO: docs.
    fn is_dir<P: AsRef<AbsPath> + Send>(
        &self,
        path: P,
    ) -> impl Future<Output = Result<bool, Self::NodeAtPathError>> {
        async move {
            self.node_at_path(path).await.map(|maybe_node| {
                maybe_node.map(|node| node.is_dir()).unwrap_or(false)
            })
        }
    }

    /// TODO: docs.
    fn is_file<P: AsRef<AbsPath> + Send>(
        &self,
        path: P,
    ) -> impl Future<Output = Result<bool, Self::NodeAtPathError>> {
        async move {
            self.node_at_path(path).await.map(|maybe_node| {
                maybe_node.map(|node| node.is_dir()).unwrap_or(false)
            })
        }
    }

    /// TODO: docs.
    #[inline]
    fn dir<P: AsRef<AbsPath> + Send>(
        &self,
        path: P,
    ) -> impl Future<Output = Result<Self::Directory, GetDirError<Self>>> + Send
    {
        async move {
            let path = path.as_ref();

            match self
                .node_at_path(path)
                .await
                .map_err(GetDirError::GetNode)?
                .ok_or_else(|| GetDirError::NoNodeAtPath(path.to_owned()))?
            {
                FsNode::File(file) => Err(GetDirError::GotFile(file)),
                FsNode::Directory(dir) => Ok(dir),
                FsNode::Symlink(symlink) => {
                    Err(GetDirError::GotSymlink(symlink))
                },
            }
        }
    }

    /// TODO: docs.
    #[inline]
    fn read_file<P: AsRef<AbsPath> + Send>(
        &self,
        path: P,
    ) -> impl Future<Output = Result<Vec<u8>, ReadFileError<Self>>> + Send
    {
        async move {
            let path = path.as_ref();

            match self
                .node_at_path(path)
                .await
                .map_err(ReadFileError::NodeAtPath)?
                .ok_or_else(|| ReadFileError::NoNodeAtPath(path.to_owned()))?
            {
                FsNode::File(file) => Some(file),
                FsNode::Directory(_) => None,
                FsNode::Symlink(symlink) => {
                    match symlink
                        .follow_recursively()
                        .await
                        .map_err(ReadFileError::FollowSymlink)?
                        .ok_or_else(|| {
                            ReadFileError::NoNodeAtPath(path.to_owned())
                        })? {
                        FsNode::File(file) => Some(file),
                        FsNode::Directory(_) => None,
                        _ => unreachable!(
                            "recursively following a symlink cannot resolve \
                             to another symlink"
                        ),
                    }
                },
            }
            .ok_or_else(|| ReadFileError::DirectoryAtPath(path.to_owned()))?
            .read()
            .await
            .map_err(ReadFileError::ReadFile)
        }
    }

    /// TODO: docs.
    #[inline]
    fn read_file_to_string<P: AsRef<AbsPath> + Send>(
        &self,
        path: P,
    ) -> impl Future<Output = Result<String, ReadFileToStringError<Self>>> + Send
    {
        async move {
            let path = path.as_ref();

            self.read_file(path)
                .await
                .map_err(ReadFileToStringError::ReadFile)
                .and_then(|contents| {
                    String::from_utf8(contents).map_err(|_| {
                        ReadFileToStringError::FileIsNotUtf8(path.to_owned())
                    })
                })
        }
    }
}

/// The type of error that can occur when trying to get the directory at a
/// given path via [`Fs::dir`].
#[derive(
    cauchy::Debug,
    derive_more::Display,
    cauchy::Error,
    cauchy::PartialEq,
    cauchy::Eq,
)]
pub enum GetDirError<Fs: self::Fs> {
    /// Getting the node at the given path failed.
    #[display("{_0}")]
    GetNode(Fs::NodeAtPathError),

    /// The node at the given path was a file, but a directory was expected.
    #[display("expected a directory at {:?}, but got a file", _0.path())]
    GotFile(Fs::File),

    /// The node at the given path was a symlink, but a directory was expected.
    #[display("expected a directory at {:?}, but got a symlink", _0.path())]
    GotSymlink(Fs::Symlink),

    /// There wasn't any node at the given path.
    #[display("no file or directory at {_0:?}")]
    NoNodeAtPath(AbsPathBuf),
}

/// TODO: docs.
#[derive(
    cauchy::Debug,
    derive_more::Display,
    cauchy::Error,
    cauchy::PartialEq,
    cauchy::Eq,
)]
pub enum ReadFileError<Fs: self::Fs> {
    /// TODO: docs.
    #[display("{_0}")]
    NodeAtPath(Fs::NodeAtPathError),

    /// TODO: docs.
    #[display("{_0}")]
    ReadFile(<Fs::File as File>::ReadError),

    /// TODO: docs.
    #[display("{_0}")]
    FollowSymlink(<Fs::Symlink as Symlink>::FollowError),

    /// TODO: docs.
    #[display("no file or directory at {_0}")]
    NoNodeAtPath(AbsPathBuf),

    /// TODO: docs.
    #[display("node at {_0} is a directory, but expected a file")]
    DirectoryAtPath(AbsPathBuf),
}

/// TODO: docs.
#[derive(
    cauchy::Debug,
    derive_more::Display,
    cauchy::Error,
    cauchy::PartialEq,
    cauchy::Eq,
)]
pub enum ReadFileToStringError<Fs: self::Fs> {
    /// TODO: docs.
    #[display("{_0}")]
    ReadFile(ReadFileError<Fs>),

    /// TODO: docs.
    #[display(
        "tried to read contents of file {_0} into a string, but it contains \
         binary data"
    )]
    FileIsNotUtf8(AbsPathBuf),
}
