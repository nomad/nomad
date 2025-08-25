use std::sync::Arc;

use bytes::Bytes;
use collab_types::fs::{
    DirectoryCreation,
    DirectoryDeletion,
    DirectoryMove,
    FileCreation,
    NewFileContents,
    Rename,
};
use collab_types::{PeerId, bytes, crop, puff};
use crop::Rope;
use puff::directory::{
    GlobalDirectoryId,
    LocalDirectoryId,
    MoveError,
    RenameError,
};
use puff::node::{Editable, IsVisible, Visible};

use crate::abs_path::{AbsPathBuf, NodeName, NodeNameBuf};
use crate::binary::BinaryContents;
use crate::fs::{
    File,
    FileContents,
    FileMut,
    Node,
    NodeMut,
    PuffChildren,
    PuffDirectory,
    PuffDirectoryMut,
    PuffNode,
};
use crate::project::{State, StateMut};
use crate::symlink::SymlinkContents;
use crate::text::TextContents;

pub type DirectoryContents = ();

/// TODO: docs.
pub struct Directory<'a, S = Visible> {
    inner: PuffDirectory<'a, S>,
    state: State<'a>,
}

/// TODO: docs.
pub struct DirectoryMut<'a, S = Editable> {
    inner: PuffDirectoryMut<'a, S>,
    state: StateMut<'a>,
}

/// TODO: docs.
pub struct Children<'a, S> {
    state: State<'a>,
    parent: PuffChildren<'a, S>,
}

impl<'a, S> Directory<'a, S> {
    /// TODO: docs.
    #[inline]
    pub fn created_by(&self) -> PeerId {
        PeerId::new(self.inner.created_by())
    }

    #[inline]
    pub(crate) fn new(
        directory: PuffDirectory<'a, S>,
        state: State<'a>,
    ) -> Self {
        Self { inner: directory, state }
    }
}

impl<'a, S: IsVisible> Directory<'a, S> {
    /// TODO: docs.
    #[inline]
    pub fn children(&self) -> Children<'a, S> {
        Children { state: self.state, parent: self.inner.children() }
    }

    /// TODO: docs.
    #[inline]
    pub fn global_id(&self) -> GlobalDirectoryId {
        self.inner.global_id()
    }

    /// TODO: docs.
    #[inline]
    pub fn id(&self) -> LocalDirectoryId {
        self.inner.local_id()
    }

    /// TODO: docs.
    #[inline]
    pub fn parent(&self) -> Option<Self> {
        self.inner.parent().map(|dir| Self::new(dir, self.state))
    }

    /// TODO: docs.
    #[inline]
    pub fn path(&self) -> AbsPathBuf {
        self.inner.path()
    }

    /// TODO: docs.
    #[inline]
    pub fn try_name(&self) -> Option<&'a NodeName> {
        self.inner.try_name()
    }
}

impl<'a, S> DirectoryMut<'a, S> {
    /// TODO: docs.
    #[inline]
    pub fn as_directory(&self) -> Directory<'_, S> {
        Directory {
            inner: self.inner.as_directory(),
            state: self.state.as_ref(),
        }
    }

    #[inline]
    pub(crate) fn new(
        directory: PuffDirectoryMut<'a, S>,
        state: StateMut<'a>,
    ) -> Self {
        Self { inner: directory, state }
    }
}

impl<S: IsVisible> DirectoryMut<'_, S> {
    /// TODO: docs.
    #[inline]
    pub fn force_rename(&mut self, new_name: NodeNameBuf) -> Rename {
        self.inner.force_rename(new_name)
    }

    /// TODO: docs.
    #[inline]
    pub fn path(&self) -> AbsPathBuf {
        self.inner.path()
    }

    /// TODO: docs.
    #[inline]
    pub fn rename(
        &mut self,
        new_name: NodeNameBuf,
    ) -> Result<Rename, RenameError> {
        self.inner.rename(new_name)
    }

    /// TODO: docs.
    #[inline]
    pub fn try_name(&self) -> Option<&NodeName> {
        self.as_directory().try_name()
    }
}

impl<'a> DirectoryMut<'a, Editable> {
    /// TODO: docs.
    #[inline]
    pub fn create_directory(
        &mut self,
        directory_name: NodeNameBuf,
    ) -> Result<(DirectoryCreation, DirectoryMut<'_>), NodeMut<'_>> {
        match self.inner.create_directory(directory_name, ()) {
            Ok((creation, directory)) => Ok((
                creation,
                DirectoryMut::new(directory, self.state.reborrow()),
            )),
            Err(node) => Err(NodeMut::new(node, self.state.reborrow())),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn create_binary_file(
        &mut self,
        file_name: NodeNameBuf,
        file_contents: impl Into<Bytes>,
    ) -> Result<(FileCreation, FileMut<'_>), NodeMut<'_>> {
        let file_contents = file_contents.into();
        let contents = FileContents::Binary(BinaryContents::new_local(
            self.state.local_id(),
            file_contents.clone(),
            self.state.binary_ctx_mut(),
        ));
        match self.inner.create_file(file_name, contents) {
            Ok((creation, file)) => Ok((
                creation
                    .map_metadata(|_| NewFileContents::Binary(file_contents)),
                FileMut::new(file, self.state.reborrow()),
            )),
            Err(node) => Err(NodeMut::new(node, self.state.reborrow())),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn create_symlink(
        &mut self,
        symlink_name: NodeNameBuf,
        symlink_target_path: impl Into<Arc<str>>,
    ) -> Result<(FileCreation, FileMut<'_>), NodeMut<'_>> {
        let target_path = symlink_target_path.into();
        let contents =
            FileContents::Symlink(SymlinkContents::new(target_path.clone()));
        match self.inner.create_file(symlink_name, contents) {
            Ok((creation, file)) => Ok((
                creation
                    .map_metadata(|_| NewFileContents::Symlink(target_path)),
                FileMut::new(file, self.state.reborrow()),
            )),
            Err(node) => Err(NodeMut::new(node, self.state.reborrow())),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn create_text_file(
        &mut self,
        file_name: NodeNameBuf,
        file_contents: impl Into<Rope>,
    ) -> Result<(FileCreation, FileMut<'_>), NodeMut<'_>> {
        let file_contents = file_contents.into();

        let contents =
            FileContents::Text(TextContents::new(file_contents.clone()));

        match self.inner.create_file(file_name, contents) {
            Ok((creation, file)) => Ok((
                creation
                    .map_metadata(|_| NewFileContents::Text(file_contents)),
                FileMut::new(file, self.state.reborrow()),
            )),
            Err(node) => Err(NodeMut::new(node, self.state.reborrow())),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn delete(self) -> Result<DirectoryDeletion, Self> {
        self.inner
            .delete()
            .map(|(deletion, _)| deletion)
            .map_err(|inner| Self::new(inner, self.state))
    }

    /// TODO: docs.
    #[inline]
    pub fn r#move(
        &mut self,
        new_parent_id: LocalDirectoryId,
    ) -> Result<DirectoryMove, MoveError> {
        self.inner.r#move(new_parent_id)
    }
}

impl<'a, S: IsVisible> Iterator for Children<'a, S> {
    type Item = Node<'a, S>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.parent.next().map(|node| match node {
            PuffNode::Directory(directory) => {
                Node::Directory(Directory::new(directory, self.state))
            },
            PuffNode::File(file) => Node::File(File::new(file, self.state)),
        })
    }
}
