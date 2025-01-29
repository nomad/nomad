use core::convert::Infallible;
use std::borrow::Cow;
use std::fs::Metadata;

use nvimx2::fs::{self, FsNodeKind, FsNodeName, FsNodeNameBuf};

use crate::{WalkDir, WalkErrorKind};

/// TODO: docs.
pub struct DirEntry<'a, W: WalkDir> {
    inner: W::DirEntry,
    name: FsNodeNameBuf,
    node_kind: FsNodeKind,
    parent_path: &'a fs::AbsPath,
}

impl<'a, W: WalkDir> DirEntry<'a, W> {
    /// TODO: docs.
    pub fn inner(&self) -> &W::DirEntry {
        &self.inner
    }

    /// TODO: docs.
    pub fn inner_mut(&mut self) -> &mut W::DirEntry {
        &mut self.inner
    }

    /// TODO: docs.
    pub fn into_inner(self) -> W::DirEntry {
        self.inner
    }

    /// TODO: docs.
    #[allow(clippy::same_name_method)]
    pub fn name(&self) -> &FsNodeName {
        &self.name
    }

    /// TODO: docs.
    #[allow(clippy::same_name_method)]
    pub fn node_kind(&self) -> FsNodeKind {
        self.node_kind
    }

    /// TODO: docs.
    pub fn parent_path(&self) -> &'a fs::AbsPath {
        self.parent_path
    }

    /// TODO: docs.
    pub fn path(&self) -> fs::AbsPathBuf {
        let mut path = self.parent_path.to_owned();
        path.push(self.name());
        path
    }

    /// TODO: docs.
    pub(crate) async fn new(
        parent_path: &'a fs::AbsPath,
        inner: W::DirEntry,
    ) -> Result<Self, WalkErrorKind<W>> {
        use fs::DirEntry;
        let node_kind = inner
            .node_kind()
            .await
            .map_err(WalkErrorKind::DirEntryNodeKind)?;
        let name = inner
            .name()
            .await
            .map(Cow::into_owned)
            .map_err(WalkErrorKind::DirEntryName)?;
        Ok(Self { inner, name, node_kind, parent_path })
    }
}

impl<W: WalkDir> fs::DirEntry for DirEntry<'_, W> {
    type MetadataError = <W::DirEntry as fs::DirEntry>::MetadataError;
    type NameError = Infallible;
    type NodeKindError = Infallible;

    async fn metadata(&self) -> Result<Metadata, Self::MetadataError> {
        self.inner.metadata().await
    }

    async fn name(&self) -> Result<Cow<'_, FsNodeName>, Self::NameError> {
        Ok(Cow::Borrowed(&self.name))
    }

    async fn node_kind(&self) -> Result<FsNodeKind, Self::NodeKindError> {
        Ok(self.node_kind)
    }
}
