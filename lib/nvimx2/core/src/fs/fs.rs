use core::error::Error;
use core::future::Future;

use futures_lite::Stream;

use crate::fs::{AbsPath, Directory, File, FsEvent, FsNode, Symlink};

/// TODO: docs.
pub trait Fs: Sized + Send + 'static {
    /// TODO: docs.
    type Directory: Directory<Fs = Self>;

    /// TODO: docs.
    type File: File<Fs = Self>;

    /// TODO: docs.
    type Symlink: Symlink<Fs = Self>;

    /// TODO: docs.
    type Timestamp: Clone + Ord;

    /// TODO: docs.
    type Watcher: Stream<
        Item = Result<FsEvent<Self::Timestamp>, Self::WatchError>,
    >;

    /// TODO: docs.
    type CreateDirectoryError: Error;

    /// TODO: docs.
    type CreateFileError: Error;

    /// TODO: docs.
    type NodeAtPathError: Error;

    /// TODO: docs.
    type WatchError: Error;

    /// TODO: docs.
    fn create_directory<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> impl Future<Output = Result<Self::Directory, Self::CreateDirectoryError>>;

    /// TODO: docs.
    fn create_file<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> impl Future<Output = Result<Self::File, Self::CreateFileError>>;

    /// TODO: docs.
    fn node_at_path<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> impl Future<Output = Result<Option<FsNode<Self>>, Self::NodeAtPathError>>;

    /// TODO: docs.
    fn now(&self) -> Self::Timestamp;

    /// TODO: docs.
    fn watch<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> impl Future<Output = Result<Self::Watcher, Self::WatchError>>;

    /// TODO: docs.
    fn exists<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> impl Future<Output = Result<bool, Self::NodeAtPathError>> {
        async move { self.node_at_path(path).await.map(|opt| opt.is_some()) }
    }

    /// TODO: docs.
    fn is_dir<P: AsRef<AbsPath>>(
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
    fn is_file<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> impl Future<Output = Result<bool, Self::NodeAtPathError>> {
        async move {
            self.node_at_path(path).await.map(|maybe_node| {
                maybe_node.map(|node| node.is_dir()).unwrap_or(false)
            })
        }
    }
}
