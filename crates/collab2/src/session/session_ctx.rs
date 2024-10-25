use core::ops::Deref;

use e31e::fs::AbsPathBuf;
use e31e::{CursorCreation, CursorId, CursorRefMut, Edit, FileId, FileRef};
use nohash::IntMap as NoHashMap;
use nomad::ctx::{BufferCtx, NeovimCtx};
use nomad::{
    ActorId,
    BufferId,
    ByteOffset,
    Replacement,
    Shared,
    ShouldDetach,
};

#[derive(Clone)]
pub(super) struct SessionCtx {
    /// The [`ActorId`] of the [`Session`].
    pub(super) actor_id: ActorId,

    /// Map from [`BufferId`]
    pub(super) buffer_actions: NoHashMap<BufferId, Shared<ShouldDetach>>,

    /// The [`CursorId`] of the cursor owned by the local peer, or `None` if
    /// it's in a buffer that's not in the project.
    pub(super) local_cursor_id: Option<CursorId>,

    /// An instance of the [`NeovimCtx`].
    pub(super) neovim_ctx: NeovimCtx<'static>,

    /// The absolute path to the root of the project.
    pub(super) project_root: AbsPathBuf,

    /// The [`Replica`](e31e::Replica) used to integrate remote messages on the
    /// project at [`project_root`](Self::project_root).
    pub(super) replica: e31e::Replica,
}

impl SessionCtx {
    /// Returns the [`FileRefMut`] corresponding to the file that's currently
    /// being edited in the buffer with the given [`BufferId`], if any.
    pub(super) fn file_of_buffer_id(
        &mut self,
        buffer_id: BufferId,
    ) -> Option<FileRef<'_>> {
        let file_ctx = self
            .neovim_ctx
            .reborrow()
            .into_buffer(buffer_id)
            .and_then(|ctx| ctx.into_file())?;

        let file_path = file_ctx.path().strip_prefix(&self.project_root)?;

        match self.replica.file_at_path(file_path) {
            Ok(Some(file)) => Some(file),
            _ => None,
        }
    }

    /// Same as [`file_of_buffer_id`](Self::file_of_buffer_id), but returns a
    /// [`FileRefMut`].
    pub(super) fn file_mut_of_buffer_id(
        &mut self,
        buffer_id: BufferId,
    ) -> Option<FileRefMut<'_>> {
        match self.file_of_buffer_id(buffer_id) {
            Some(file) => Some(FileRefMut {
                file_id: file.id(),
                buffer_id,
                session_ctx: self,
            }),
            None => None,
        }
    }

    pub(super) fn integrate_cursor_creation(
        &mut self,
        cursor_creation: CursorCreation,
    ) {
        let Some(cursor) =
            self.replica.integrate_cursor_creation(cursor_creation)
        else {
            return;
        };
        let Some(buffer) = self.buffer_of_file_id(cursor.file().id()) else {
            return;
        };

        todo!();
    }

    pub(super) fn local_cursor_mut(&mut self) -> Option<CursorRefMut<'_>> {
        self.local_cursor_id.and_then(|id| self.replica.cursor_mut(id))
    }

    /// Returns the [`BufferCtx`] of the buffer displaying the file with the
    /// given [`FileId`], if any.
    fn buffer_of_file_id(&self, file_id: FileId) -> Option<BufferCtx<'_>> {
        let file = self.replica.file(file_id)?;
        let file_path_in_project = file.path();
        let file_path = (&*self.project_root).concat(&file_path_in_project);
        let buffer_id = BufferId::of_name(&*file_path)?;
        self.neovim_ctx.reborrow().into_buffer(buffer_id)
    }
}

pub(super) struct FileRefMut<'ctx> {
    file_id: FileId,
    buffer_id: BufferId,
    session_ctx: &'ctx mut SessionCtx,
}

impl FileRefMut<'_> {
    pub(super) fn sync_created_cursor(
        &mut self,
        byte_offset: ByteOffset,
    ) -> CursorCreation {
        assert!(
            self.session_ctx.local_cursor_id.is_none(),
            "creating a new cursor when another already exists, but Neovim \
             only supports a single cursor"
        );
        let (cursor_id, cursor_creation) =
            self.as_inner_mut().sync_created_cursor(byte_offset.into_u64());
        self.session_ctx.local_cursor_id = Some(cursor_id);
        cursor_creation
    }

    pub(super) fn sync_edited_text(
        &mut self,
        replacement: Replacement,
    ) -> Edit {
        let edit = self.as_inner_mut().sync_edited_text([replacement.into()]);
        // TODO: for all windows displaying the buffer, update tooltips of all
        // remote cursors and selections on the buffer.
        edit
    }

    fn as_inner_mut(&mut self) -> e31e::FileRefMut<'_> {
        self.session_ctx
            .replica
            .file_mut(self.file_id)
            .expect("we have a FileRefMut")
    }
}
