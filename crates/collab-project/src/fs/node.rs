use collab_types::fs::Rename;
use collab_types::puff::node::IsVisible;
use collab_types::{PeerId, puff};
use puff::directory::RenameError;
use puff::node::{Editable, Visible};

use crate::abs_path::{AbsPathBuf, NodeName, NodeNameBuf};
use crate::fs::{
    Directory,
    DirectoryMut,
    File,
    FileMut,
    PuffNode,
    PuffNodeMut,
};
use crate::project::{State, StateMut};

/// TODO: docs.
pub enum Node<'a, S = Visible> {
    /// TODO: docs.
    Directory(Directory<'a, S>),

    /// TODO: docs.
    File(File<'a, S>),
}

/// TODO: docs.
pub enum NodeMut<'a, S = Editable> {
    /// TODO: docs.
    Directory(DirectoryMut<'a, S>),

    /// TODO: docs.
    File(FileMut<'a, S>),
}

impl<'a, S> Node<'a, S> {
    /// TODO: docs.
    #[inline]
    pub fn created_by(&self) -> PeerId {
        match self {
            Self::Directory(directory) => directory.created_by(),
            Self::File(file) => file.created_by(),
        }
    }

    /// TODO: docs.
    pub fn unwrap_directory(self) -> Directory<'a, S> {
        match self {
            Self::Directory(directory) => directory,
            Self::File(_) => {
                panic!("called `Node::unwrap_directory()` on a file")
            },
        }
    }

    /// TODO: docs.
    pub fn unwrap_file(self) -> File<'a, S> {
        match self {
            Self::File(file) => file,
            Self::Directory(_) => {
                panic!("called `Node::unwrap_file()` on a directory")
            },
        }
    }

    #[inline]
    pub(crate) fn new(node: PuffNode<'a, S>, state: State<'a>) -> Self {
        match node {
            PuffNode::Directory(directory) => {
                Self::Directory(Directory::new(directory, state))
            },
            PuffNode::File(file) => Self::File(File::new(file, state)),
        }
    }
}

impl<'a, S: IsVisible> Node<'a, S> {
    /// Returns the node's path project.
    #[inline]
    pub fn path(&self) -> AbsPathBuf {
        match self {
            Self::Directory(directory) => directory.path(),
            Self::File(file) => file.path(),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn try_name(&self) -> Option<&'a NodeName> {
        match self {
            Self::Directory(directory) => directory.try_name(),
            Self::File(file) => Some(file.name()),
        }
    }
}

impl<'a, S> NodeMut<'a, S> {
    /// TODO: docs.
    pub fn unwrap_directory(self) -> DirectoryMut<'a, S> {
        match self {
            Self::Directory(directory) => directory,
            Self::File(_) => {
                panic!("called `NodeMut::unwrap_directory()` on a file")
            },
        }
    }

    /// TODO: docs.
    pub fn unwrap_file(self) -> FileMut<'a, S> {
        match self {
            Self::File(file) => file,
            Self::Directory(_) => {
                panic!("called `NodeMut::unwrap_file()` on a directory")
            },
        }
    }

    #[inline]
    pub(crate) fn new(node: PuffNodeMut<'a, S>, state: StateMut<'a>) -> Self {
        match node {
            PuffNodeMut::Directory(directory) => {
                Self::Directory(DirectoryMut::new(directory, state))
            },
            PuffNodeMut::File(file) => Self::File(FileMut::new(file, state)),
        }
    }
}

impl<S: IsVisible> NodeMut<'_, S> {
    /// TODO: docs.
    #[inline]
    pub fn force_rename(&mut self, new_name: NodeNameBuf) -> Rename {
        match self {
            Self::Directory(dir) => dir.force_rename(new_name),
            Self::File(file) => file.force_rename(new_name),
        }
    }

    /// Returns the node's path project.
    #[inline]
    pub fn path(&self) -> AbsPathBuf {
        match self {
            Self::Directory(directory) => directory.path(),
            Self::File(file) => file.path(),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn rename(
        &mut self,
        new_name: NodeNameBuf,
    ) -> Result<Rename, RenameError> {
        match self {
            Self::Directory(dir) => dir.rename(new_name),
            Self::File(file) => {
                file.rename(new_name).map_err(|err| match err {
                    puff::file::RenameError::NameConflicts(local_node_id) => {
                        RenameError::NameConflicts(local_node_id)
                    },
                })
            },
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn try_name(&self) -> Option<&NodeName> {
        match self {
            Self::Directory(directory) => directory.try_name(),
            Self::File(file) => Some(file.name()),
        }
    }
}
