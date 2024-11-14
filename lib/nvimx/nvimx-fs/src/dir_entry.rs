use alloc::borrow::Cow;
use core::error::Error;
use core::future::Future;

use crate::{FsNodeKind, FsNodeName};

/// TODO: docs.
pub trait DirEntry {
    /// TODO: docs.
    type NameError: Error;

    /// TODO: docs.
    type NodeKindError: Error;

    /// TODO: docs.
    fn name(
        &self,
    ) -> impl Future<Output = Result<Cow<'_, FsNodeName>, Self::NameError>>;

    /// TODO: docs.
    fn node_kind(
        &self,
    ) -> impl Future<Output = Result<FsNodeKind, Self::NodeKindError>>;

    /// TODO: docs.
    fn is_directory(
        &self,
    ) -> impl Future<Output = Result<bool, Self::NodeKindError>> {
        async { self.node_kind().await.map(|k| k.is_directory()) }
    }

    /// TODO: docs.
    fn is_file(
        &self,
    ) -> impl Future<Output = Result<bool, Self::NodeKindError>> {
        async { self.node_kind().await.map(|k| k.is_file()) }
    }
}
