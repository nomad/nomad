use collab_types::fs::{FileDeletion, FileMove, Rename};
use collab_types::{PeerId, puff};
use puff::directory::LocalDirectoryId;
use puff::file::{GlobalFileId, LocalFileId, MoveError, RenameError};
use puff::node::{Editable, IsVisible, Visible};

use crate::abs_path::{AbsPathBuf, NodeName, NodeNameBuf};
use crate::binary::{BinaryContents, BinaryFile, BinaryFileMut};
use crate::fs::{Directory, PuffFile, PuffFileMut};
use crate::project::{State, StateMut};
use crate::symlink::{SymlinkContents, SymlinkFile, SymlinkFileMut};
use crate::text::{TextContents, TextFile, TextFileMut};

#[derive(Clone)]
#[allow(private_interfaces)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FileContents {
    Binary(BinaryContents),
    Symlink(SymlinkContents),
    Text(Box<TextContents>),
}

/// TODO: docs.
pub enum File<'a, S = Visible> {
    /// TODO: docs.
    Binary(BinaryFile<'a, S>),

    /// TODO: docs.
    Symlink(SymlinkFile<'a, S>),

    /// TODO: docs.
    Text(TextFile<'a, S>),
}

/// TODO: docs.
pub enum FileMut<'a, S = Editable> {
    /// TODO: docs.
    Binary(BinaryFileMut<'a, S>),

    /// TODO: docs.
    Symlink(SymlinkFileMut<'a, S>),

    /// TODO: docs.
    Text(TextFileMut<'a, S>),
}

impl FileContents {
    #[inline]
    pub(crate) fn is_binary(&self) -> bool {
        matches!(self, Self::Binary(_))
    }

    #[inline]
    pub(crate) fn is_symlink(&self) -> bool {
        matches!(self, Self::Symlink(_))
    }

    #[inline]
    pub(crate) fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }
}

impl<'a, S> File<'a, S> {
    /// TODO: docs.
    #[inline]
    pub fn created_by(&self) -> PeerId {
        PeerId::new(self.inner().created_by())
    }

    /// TODO: docs.
    #[inline]
    pub fn name(&self) -> &'a NodeName {
        self.inner().name()
    }

    /// TODO: docs.
    pub fn unwrap_binary(self) -> BinaryFile<'a, S> {
        match self {
            Self::Binary(file) => file,
            _ => panic!("called `File::unwrap_binary()` on a non-binary file"),
        }
    }

    /// TODO: docs.
    pub fn unwrap_symlink(self) -> SymlinkFile<'a, S> {
        match self {
            Self::Symlink(file) => file,
            _ => {
                panic!("called `File::unwrap_symlink()` on a non-symlink file")
            },
        }
    }

    /// TODO: docs.
    pub fn unwrap_text(self) -> TextFile<'a, S> {
        match self {
            Self::Text(file) => file,
            _ => panic!("called `File::unwrap_text()` on a non-text file"),
        }
    }

    #[inline]
    pub(crate) fn new(file: PuffFile<'a, S>, state: State<'a>) -> Self {
        match file.metadata() {
            FileContents::Binary(_) => {
                Self::Binary(BinaryFile::new(file, state))
            },
            FileContents::Symlink(_) => {
                Self::Symlink(SymlinkFile::new(file, state))
            },
            FileContents::Text(_) => Self::Text(TextFile::new(file, state)),
        }
    }

    #[inline]
    fn inner(&self) -> PuffFile<'a, S> {
        match self {
            Self::Binary(file) => file.inner(),
            Self::Symlink(file) => file.inner(),
            Self::Text(file) => file.inner(),
        }
    }

    #[inline]
    fn state(&self) -> State<'a> {
        match self {
            Self::Binary(file) => file.state(),
            Self::Symlink(file) => file.state(),
            Self::Text(file) => file.state(),
        }
    }
}

impl<'a, S: IsVisible> File<'a, S> {
    /// TODO: docs.
    #[inline]
    pub fn global_id(&self) -> GlobalFileId {
        self.inner().global_id()
    }

    /// TODO: docs.
    #[inline]
    pub fn id(&self) -> LocalFileId {
        self.inner().local_id()
    }

    /// TODO: docs.
    #[inline]
    pub fn parent(&self) -> Directory<'a, S> {
        Directory::new(self.inner().parent(), self.state())
    }
}

impl<'a, S: IsVisible> File<'a, S> {
    /// TODO: docs.
    #[inline]
    pub fn path(&self) -> AbsPathBuf {
        self.inner().path()
    }
}

impl<'a, S> FileMut<'a, S> {
    /// TODO: docs.
    #[inline]
    pub fn as_file(&self) -> File<'_, S> {
        match self {
            Self::Binary(file) => File::Binary(file.as_file()),
            Self::Symlink(file) => File::Symlink(file.as_file()),
            Self::Text(file) => File::Text(file.as_file()),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn name(&self) -> &NodeName {
        self.as_file().name()
    }

    /// TODO: docs.
    pub fn unwrap_binary(self) -> BinaryFileMut<'a, S> {
        match self {
            Self::Binary(file) => file,
            _ => panic!(
                "called `FileMut::unwrap_binary()` on a non-binary file"
            ),
        }
    }

    /// TODO: docs.
    pub fn unwrap_symlink(self) -> SymlinkFileMut<'a, S> {
        match self {
            Self::Symlink(file) => file,
            _ => {
                panic!(
                    "called `FileMut::unwrap_symlink()` on a non-symlink file"
                )
            },
        }
    }

    /// TODO: docs.
    pub fn unwrap_text(self) -> TextFileMut<'a, S> {
        match self {
            Self::Text(file) => file,
            _ => panic!("called `FileMut::unwrap_text()` on a non-text file"),
        }
    }

    #[inline]
    pub(crate) fn new(file: PuffFileMut<'a, S>, state: StateMut<'a>) -> Self {
        match file.metadata() {
            FileContents::Binary(_) => {
                Self::Binary(BinaryFileMut::new(file, state))
            },
            FileContents::Symlink(_) => {
                Self::Symlink(SymlinkFileMut::new(file, state))
            },
            FileContents::Text(_) => Self::Text(TextFileMut::new(file, state)),
        }
    }

    #[inline]
    fn inner_mut(&mut self) -> &mut PuffFileMut<'a, S> {
        match self {
            Self::Binary(file) => file.inner_mut(),
            Self::Symlink(file) => file.inner_mut(),
            Self::Text(file) => file.inner_mut(),
        }
    }

    #[inline]
    fn into_inner(self) -> PuffFileMut<'a, S> {
        match self {
            Self::Binary(file) => file.into_inner(),
            Self::Symlink(file) => file.into_inner(),
            Self::Text(file) => file.into_inner(),
        }
    }
}

impl<S: IsVisible> FileMut<'_, S> {
    /// TODO: docs.
    #[inline]
    pub fn force_rename(&mut self, new_name: NodeNameBuf) -> Rename {
        self.inner_mut().force_rename(new_name)
    }

    /// TODO: docs.
    #[inline]
    pub fn path(&self) -> AbsPathBuf {
        self.as_file().path()
    }

    /// TODO: docs.
    #[inline]
    pub fn rename(
        &mut self,
        new_name: NodeNameBuf,
    ) -> Result<Rename, RenameError> {
        self.inner_mut().rename(new_name)
    }
}

impl<'a> FileMut<'a, Editable> {
    /// TODO: docs.
    #[inline]
    pub fn delete(self) -> FileDeletion {
        self.into_inner().delete().0
    }

    /// TODO: docs.
    #[inline]
    pub fn r#move(
        &mut self,
        new_parent_id: LocalDirectoryId,
    ) -> Result<FileMove, MoveError> {
        self.inner_mut().r#move(new_parent_id)
    }
}

impl<'a, Ctx> Copy for File<'a, Ctx> {}

impl<'a, Ctx> Clone for File<'a, Ctx> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
