use collab_types::PeerId;
use collab_types::fs::{
    DirectoryCreation,
    DirectoryDeletion,
    DirectoryMove,
    FileCreation,
    FileDeletion,
    FileMove,
    NewFileContents,
    Rename,
};

use crate::Project;
use crate::binary::BinaryContents;
use crate::fs::{FileContents, SyncActions};
use crate::symlink::SymlinkContents;
use crate::text::TextContents;

/// TODO: docs.
pub trait FsOp: Sized + private::Sealed {
    #[doc(hidden)]
    fn integrate_into(self, proj: &mut Project) -> SyncActions<'_>;
}

impl FsOp for FileCreation {
    #[inline]
    fn integrate_into(self, proj: &mut Project) -> SyncActions<'_> {
        let created_by = PeerId::new(self.performed_by());

        let creation = self.map_metadata(|contents| match contents {
            NewFileContents::Binary(bytes) => FileContents::Binary(
                BinaryContents::new_remote(bytes, created_by),
            ),
            NewFileContents::Symlink(target_path) => {
                FileContents::Symlink(SymlinkContents::new(target_path))
            },
            NewFileContents::Text(text) => {
                FileContents::Text(TextContents::new(text))
            },
        });

        let (state, fs) = proj.state_mut();
        SyncActions::new(fs.integrate_file_creation(creation), state)
    }
}

impl FsOp for DirectoryCreation {
    #[doc(hidden)]
    fn integrate_into(self, proj: &mut Project) -> SyncActions<'_> {
        let (state, fs) = proj.state_mut();
        SyncActions::new(fs.integrate_directory_creation(self), state)
    }
}

impl FsOp for DirectoryDeletion {
    #[doc(hidden)]
    fn integrate_into(self, proj: &mut Project) -> SyncActions<'_> {
        let (state, fs) = proj.state_mut();
        SyncActions::new(fs.integrate_directory_deletion(self), state)
    }
}

impl FsOp for DirectoryMove {
    #[doc(hidden)]
    fn integrate_into(self, proj: &mut Project) -> SyncActions<'_> {
        let (state, fs) = proj.state_mut();
        SyncActions::new(fs.integrate_directory_move(self), state)
    }
}

impl FsOp for FileDeletion {
    #[doc(hidden)]
    fn integrate_into(self, proj: &mut Project) -> SyncActions<'_> {
        let (state, fs) = proj.state_mut();
        SyncActions::new(fs.integrate_file_deletion(self), state)
    }
}

impl FsOp for FileMove {
    #[doc(hidden)]
    fn integrate_into(self, proj: &mut Project) -> SyncActions<'_> {
        let (state, fs) = proj.state_mut();
        SyncActions::new(fs.integrate_file_move(self), state)
    }
}

impl FsOp for Rename {
    #[doc(hidden)]
    fn integrate_into(self, proj: &mut Project) -> SyncActions<'_> {
        let (state, fs) = proj.state_mut();
        SyncActions::new(fs.integrate_rename(self), state)
    }
}

mod private {
    use super::*;

    pub trait Sealed {}

    impl Sealed for FileCreation {}
    impl Sealed for DirectoryCreation {}
    impl Sealed for DirectoryDeletion {}
    impl Sealed for DirectoryMove {}
    impl Sealed for FileDeletion {}
    impl Sealed for FileMove {}
    impl Sealed for Rename {}
}
