use core::error::Error;
use core::future::Future;

use futures_lite::Stream;

use crate::fs::{AbsPath, DirEntry, FsNode, Watcher};

/// TODO: docs.
pub trait Fs: Sized + 'static {
    /// TODO: docs.
    type Timestamp: Clone + Ord;

    /// TODO: docs.
    type DirEntry: DirEntry;

    /// TODO: docs.
    type Directory<Path>;

    /// TODO: docs.
    type File<Path>;

    /// TODO: docs.
    type ReadDir: Stream<Item = Result<Self::DirEntry, Self::DirEntryError>>;

    /// TODO: docs.
    type DirEntryError: Error;

    /// TODO: docs.
    type NodeAtPathError: Error;

    /// TODO: docs.
    type ReadDirError: Error;

    /// TODO: docs.
    type Watcher: Watcher<Self>;

    /// TODO: docs.
    type WatchError: Error;

    /// TODO: docs.
    fn node_at_path<P: AsRef<AbsPath>>(
        &mut self,
        path: P,
    ) -> impl Future<Output = Result<Option<FsNode<Self, P>>, Self::NodeAtPathError>>;

    /// TODO: docs.
    fn now(&self) -> Self::Timestamp;

    /// TODO: docs.
    fn read_dir<P: AsRef<AbsPath>>(
        &mut self,
        dir_path: P,
    ) -> impl Future<Output = Result<Self::ReadDir, Self::ReadDirError>>;

    /// TODO: docs.
    fn watch<P: AsRef<AbsPath>>(
        &mut self,
        path: P,
    ) -> impl Future<Output = Result<Self::Watcher, Self::WatchError>>;

    /// TODO: docs.
    fn exists<P: AsRef<AbsPath>>(
        &mut self,
        path: P,
    ) -> impl Future<Output = Result<bool, Self::NodeAtPathError>> {
        async move { self.node_at_path(path).await.map(|opt| opt.is_some()) }
    }

    /// TODO: docs.
    fn is_dir<P: AsRef<AbsPath>>(
        &mut self,
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
        &mut self,
        path: P,
    ) -> impl Future<Output = Result<bool, Self::NodeAtPathError>> {
        async move {
            self.node_at_path(path).await.map(|maybe_node| {
                maybe_node.map(|node| node.is_dir()).unwrap_or(false)
            })
        }
    }
}
