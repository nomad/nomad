use core::error::Error;
use core::fmt::Debug;
use core::future::Future;
use core::hash::Hash;

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
}
