use core::fmt;

use collab_server::message::Peer;
use e31e::fs::{AbsPath, AbsPathBuf};
use e31e::{
    CursorCreation,
    CursorId,
    CursorRefMut,
    CursorRelocation,
    CursorRemoval,
    Edit,
    FileId,
    FileRef,
    FileRefMut,
    Hunks,
    PeerId,
    Replica,
    SelectionCreation,
    SelectionId,
    SelectionRelocation,
    SelectionRemoval,
};
use fxhash::FxHashMap;
use nohash::IntMap as NoHashMap;
use nomad::ctx::{BufferCtx, NeovimCtx};
use nomad::{ActorId, BufferId, Replacement, Shared, ShouldDetach};

use super::{PeerSelection, PeerTooltip};

pub(crate) struct Project {
    /// The [`ActorId`] of the [`Session`].
    pub(super) actor_id: ActorId,

    /// Map from [`BufferId`]..
    pub(super) buffer_actions: NoHashMap<BufferId, Shared<ShouldDetach>>,

    /// The [`CursorId`] of the cursor owned by the local peer, or `None` if
    /// it's in a buffer that's not in the project.
    pub(super) local_cursor_id: Option<CursorId>,

    /// The local [`Peer`].
    pub(super) local_peer: Peer,

    /// An instance of the [`NeovimCtx`].
    pub(super) neovim_ctx: NeovimCtx<'static>,

    /// The absolute path to the root of the project.
    pub(super) project_root: AbsPathBuf,

    /// Map from [`PeerId`] to the corresponding remote [`Peer`].
    ///
    /// It doesn't include the local peer.
    pub(super) remote_peers: NoHashMap<PeerId, Peer>,

    /// Map from the [`SelectionId`] of a selection owned by a remote peer to
    /// the corresponding [`PeerSelection`] displayed in the editor, if any.
    pub(super) remote_selections: FxHashMap<SelectionId, PeerSelection>,

    /// Map from the [`CursorId`] of a cursor owned by a remote peer to the
    /// corresponding [`PeerTooltip`] displayed in the editor, if any.
    pub(super) remote_tooltips: FxHashMap<CursorId, PeerTooltip>,

    /// The [`Replica`] used to integrate remote messages on the project at
    /// [`project_root`](Self::project_root).
    pub(super) replica: Replica,
}

impl Project {
    /// Returns an iterator over all the peers [`Peer`]s.
    ///
    /// Note that the local peer is included in the iterator. If you don't want
    /// it be, use [`remote_peers`](Self::remote_peers) instead.
    pub(crate) fn all_peers(&self) -> impl Iterator<Item = &Peer> {
        self.remote_peers.values().chain(core::iter::once(&self.local_peer))
    }

    /// Returns an iterator over the remote [`Peer`]s.
    ///
    /// Note that the local peer is not included in the iterator. If you want
    /// it be, use [`all_peers`](Self::all_peers) instead.
    pub(crate) fn remote_peers(&self) -> impl Iterator<Item = &Peer> {
        self.remote_peers.values()
    }

    /// Returns the absolute path to the root of the project.
    pub(crate) fn root(&self) -> &AbsPath {
        &self.project_root
    }

    /// Returns the [`BufferCtx`] of the buffer displaying the file with the
    /// given [`FileId`], if any.
    pub(super) fn buffer_of_file_id(
        &self,
        file_id: FileId,
    ) -> Option<BufferCtx<'_>> {
        let file = self.replica.file(file_id)?;
        let file_path_in_project = file.path();
        let file_path = (*self.project_root).concat(&file_path_in_project);
        let buffer_id = BufferId::of_name(&*file_path)?;
        self.neovim_ctx.reborrow().into_buffer(buffer_id)
    }

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
            Some(file) => {
                let file_id = file.id();
                Some(
                    self.replica
                        .file_mut(file_id)
                        .expect("we just had a FileRef"),
                )
            },
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
        let Some(peer) = self.remote_peers.get(&cursor.owner().id()).cloned()
        else {
            return;
        };
        let cursor_id = cursor.id();
        let cursor_offset = cursor.byte_offset().into();
        let file_id = cursor.file().id();
        let Some(buffer) = self.buffer_of_file_id(file_id) else {
            return;
        };
        let peer_tooltip = PeerTooltip::create(peer, cursor_offset, buffer);
        self.remote_tooltips.insert(cursor_id, peer_tooltip);
    }

    pub(super) fn integrate_cursor_relocation(
        &mut self,
        cursor_relocation: CursorRelocation,
    ) {
        let Some(cursor) =
            self.replica.integrate_cursor_relocation(cursor_relocation)
        else {
            return;
        };
        let Some(tooltip) = self.remote_tooltips.get_mut(&cursor.id()) else {
            return;
        };
        tooltip.relocate(cursor.byte_offset().into());
    }

    pub(super) fn integrate_cursor_removal(
        &mut self,
        cursor_removal: CursorRemoval,
    ) {
        let Some(cursor_id) =
            self.replica.integrate_cursor_removal(cursor_removal)
        else {
            return;
        };
        let _ = self.remote_tooltips.remove(&cursor_id);
    }

    /// Tries to integrate the given [`Edit`] into corresponding buffer.
    ///
    /// If there's no open buffer for the file being edited, its absolute path
    /// is returned together with the [`Replacements`](Hunks) that need to be
    /// applied to it.
    pub(super) fn integrate_edit(
        &mut self,
        edit: Edit,
    ) -> Option<(AbsPathBuf, Hunks)> {
        let (file, hunks) = self.replica.integrate_edit(edit)?;
        let file_id = file.id();
        let Some(buffer) = self.buffer_of_file_id(file_id) else {
            let file_path_in_project = self
                .replica
                .file(file_id)
                .expect("we just had a FileRef")
                .path();
            let file_path = (*self.project_root)
                .concat(&file_path_in_project)
                .into_owned();
            return Some((file_path, hunks));
        };
        let text_buffer = buffer
            .into_text_buffer()
            .expect("the file is in the Replica, so it must contain text");
        for replacement in hunks.map(Replacement::from) {
            text_buffer.replace_text(
                replacement.deleted_range(),
                replacement.inserted_text(),
                self.actor_id,
            );
        }
        self.refresh_cursors(file_id);
        self.refresh_selections(file_id);
        None
    }

    pub(super) fn integrate_peer_joined(&mut self, peer: Peer) {
        assert_ne!(peer.id(), self.replica.id());
        assert!(self.remote_peers.insert(peer.id(), peer).is_none());
        // TODO: display backlogged cursors and selections.
        //
        // let peer = self.replica.peer();
        //
        // for cursor in peer.cursors() {
        //     let Some(buffer) = self.buffer_of_file_id(cursor.file().id());
        //     let tooltip = PeerTooltip::create(peer.clone() cursor.offset(), buffer);
        //     self.remote_tooltips.insert(cursor_id, peer_tooltip);
        // }
    }

    pub(super) fn integrate_peer_left(&mut self, peer_id: PeerId) {
        // TODO: allow the Replica to remove all cursors and selections owned by
        // the peer.
        todo!();
    }

    pub(super) fn integrate_selection_creation(
        &mut self,
        selection_creation: SelectionCreation,
    ) {
        let Some(selection) =
            self.replica.integrate_selection_creation(selection_creation)
        else {
            return;
        };
        if !self.remote_peers.contains_key(&selection.owner().id()) {
            return;
        }
        let selection_id = selection.id();
        let selection_range = {
            let r = selection.byte_range();
            r.start.into()..r.end.into()
        };
        let file_id = selection.file().id();
        let Some(buffer) = self.buffer_of_file_id(file_id) else {
            return;
        };
        let peer_selection = PeerSelection::create(selection_range, buffer);
        self.remote_selections.insert(selection_id, peer_selection);
    }

    pub(super) fn integrate_selection_relocation(
        &mut self,
        selection_relocation: SelectionRelocation,
    ) {
        let Some(selection) =
            self.replica.integrate_selection_relocation(selection_relocation)
        else {
            return;
        };
        let Some(peer_selection) =
            self.remote_selections.get_mut(&selection.id())
        else {
            return;
        };
        let new_range = {
            let r = selection.byte_range();
            r.start.into()..r.end.into()
        };
        peer_selection.relocate(new_range);
    }

    pub(super) fn integrate_selection_removal(
        &mut self,
        selection_removal: SelectionRemoval,
    ) {
        let Some(selection_id) =
            self.replica.integrate_selection_removal(selection_removal)
        else {
            return;
        };
        let _ = self.remote_selections.remove(&selection_id);
    }

    pub(super) fn local_cursor_mut(&mut self) -> Option<CursorRefMut<'_>> {
        self.local_cursor_id.and_then(|id| self.replica.cursor_mut(id))
    }

    pub(super) fn refresh_cursors(&mut self, file_id: FileId) {
        for cursor in self
            .replica
            .cursors()
            .filter(|c| c.file().id() == file_id)
            .filter(|c| c.owner().id() != self.replica.id())
        {
            let tooltip = self.remote_tooltips.get_mut(&cursor.id()).expect(
                "the cursor is in this file and owned by a remote peer, so \
                 it must have a tooltip",
            );
            tooltip.relocate(cursor.byte_offset().into());
        }
    }

    pub(super) fn refresh_selections(&mut self, file_id: FileId) {
        for selection in self
            .replica
            .selections()
            .filter(|s| s.file().id() == file_id)
            .filter(|s| s.owner().id() != self.replica.id())
        {
            let peer_selection =
                self.remote_selections.get_mut(&selection.id()).expect(
                    "the selection is in this file and owned by a remote \
                     peer, so it must have a peer selection",
                );
            let selection_range = {
                let r = selection.byte_range();
                r.start.into()..r.end.into()
            };
            peer_selection.relocate(selection_range);
        }
    }
}

impl fmt::Debug for Project {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Project").field(&self.project_root).finish()
    }
}
