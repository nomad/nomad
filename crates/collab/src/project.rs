//! TODO: docs.

use core::iter;
use std::sync::Arc;

use abs_path::{AbsPath, AbsPathBuf};
use collab_project::fs::{File, FileMut, FsOp, Node, NodeMut};
use collab_project::text::{CursorId, SelectionId, TextReplacement};
use collab_types::{Message, Peer, PeerId, binary, crop, puff, text};
use editor::{Access, AccessMut, AgentId, Buffer, Context, Editor};
use fs::{File as _, Fs as _, Symlink as _};
use futures_util::FutureExt;
use fxhash::FxHashMap;
use puff::directory::LocalDirectoryId;
use puff::file::{GlobalFileId, LocalFileId};
use puff::ops::Rename;
use smallvec::SmallVec;

use crate::CollabEditor;
use crate::convert::Convert;
use crate::event::{self, Event};
use crate::peers::RemotePeers;

/// TODO: docs.
pub struct Project<Ed: CollabEditor> {
    /// TODO: docs.
    pub agent_id: AgentId,

    /// Contains various mappings between editor IDs and project IDs.
    pub id_maps: IdMaps<Ed>,

    /// The inner CRDT holding the entire state of the project.
    pub inner: collab_project::Project,

    /// TODO: docs.
    pub local_peer: Peer,

    /// Map from a remote selections's ID to the corresponding selection
    /// displayed in the editor.
    pub peer_selections: FxHashMap<SelectionId, Ed::PeerSelection>,

    /// Map from a remote cursor's ID to the corresponding tooltip displayed in
    /// the editor.
    pub peer_tooltips: FxHashMap<CursorId, Ed::PeerTooltip>,

    /// The remote peers currently in the session.
    pub remote_peers: RemotePeers,

    /// The path to the root of the project.
    pub root_path: AbsPathBuf,
}

#[derive(cauchy::Default)]
#[doc(hidden)]
pub struct IdMaps<Ed: Editor> {
    pub(crate) buffer2file: FxHashMap<Ed::BufferId, LocalFileId>,
    pub(crate) cursor2cursor: FxHashMap<Ed::CursorId, CursorId>,
    pub(crate) file2buffer: FxHashMap<LocalFileId, Ed::BufferId>,
    pub(crate) node2dir:
        FxHashMap<<Ed::Fs as fs::Fs>::NodeId, LocalDirectoryId>,
    pub(crate) node2file: FxHashMap<<Ed::Fs as fs::Fs>::NodeId, LocalFileId>,
    pub(crate) selection2selection: FxHashMap<Ed::SelectionId, SelectionId>,
}

/// The type of error that can occcur when integrating a [`Message`].
#[derive(cauchy::Debug, derive_more::Display, cauchy::Error)]
#[display("{_0}")]
pub enum IntegrateError<Ed: CollabEditor> {
    /// TODO: docs..
    BinaryEdit(IntegrateBinaryEditError<Ed::Fs>),

    /// TODO: docs..
    CreateBuffer(Ed::CreateBufferError),

    /// TODO: docs..
    FsOp(IntegrateFsOpError<Ed::Fs>),
}

/// The type of error that can occcur when integrating a
/// [`binary::BinaryEdit`].
#[derive(cauchy::Debug, derive_more::Display, cauchy::Error)]
#[display("{_0}")]
pub enum IntegrateBinaryEditError<Fs: fs::Fs> {
    /// The node at the given path was a directory, not a file.
    #[display("the node at {_0} is a directory, not a file")]
    DirectoryAtPath(AbsPathBuf),

    /// It wasn't possible to get the node at the given path.
    NodeAtPath(Fs::NodeAtPathError),

    /// There wasn't any node at the given path.
    #[display("no file or directory at {_0}")]
    NoNodeAtPath(AbsPathBuf),

    /// The node at the given path was a symlink, not a file.
    #[display("the node at {_0} is a symlink, not a file")]
    SymlinkAtPath(AbsPathBuf),

    /// It wasn't possible to write the new contents of a file.
    WriteToFile(<Fs::File as fs::File>::WriteError),
}

/// The type of error that can occcur when integrating a
/// [`binary::BinaryEdit`].
#[derive(cauchy::Debug, derive_more::Display, cauchy::Error)]
#[display("{_0}")]
pub enum IntegrateFsOpError<Fs: fs::Fs> {
    /// It wasn't possible to create a directory.
    CreateDirectory(<Fs::Directory as fs::Directory>::CreateDirectoryError),

    /// It wasn't possible to create a file.
    CreateFile(<Fs::Directory as fs::Directory>::CreateFileError),

    /// It wasn't possible to create a symlink.
    CreateSymlink(<Fs::Directory as fs::Directory>::CreateSymlinkError),

    /// It wasn't possible to delete a node.
    DeleteNode(fs::DeleteNodeError<Fs>),

    /// It wasn't possible to get the directory at a particular path.
    GetDir(fs::GetDirError<Fs>),

    /// It wasn't possible to move a node to a new location.
    MoveNode(fs::MoveNodeError<Fs>),

    /// It wasn't possible to write to a file.
    WriteFile(<Fs::File as fs::File>::WriteError),
}

/// TODO: docs.
#[derive(cauchy::Debug, derive_more::Display, cauchy::Error)]
#[display("{_0}")]
pub enum SynchronizeError<Ed: CollabEditor> {
    /// TODO: docs.
    ContentsAtPath(ContentsAtPathError<Ed::Fs>),
}

/// TODO: docs.
#[derive(cauchy::Debug, derive_more::Display, cauchy::Error)]
#[display("{_0}")]
pub enum ContentsAtPathError<Fs: fs::Fs> {
    /// TODO: docs.
    NodeAtPath(Fs::NodeAtPathError),

    /// TODO: docs.
    ReadFile(<Fs::File as fs::File>::ReadError),

    /// TODO: docs.
    ReadSymlink(<Fs::Symlink as fs::Symlink>::ReadError),
}

/// The iterator returned by [`Project::integrate`].
enum Messages {
    None,
    ProjectResponse(Option<collab_types::ProjectResponse>),
    Renames(smallvec::IntoIter<[Rename; 2]>),
}

enum FsNodeContents {
    Directory,
    Text(String),
    Binary(Vec<u8>),
    Symlink(String),
}

impl<Ed: CollabEditor> Project<Ed> {
    /// TODO: docs.
    #[allow(clippy::too_many_lines)]
    pub(crate) async fn integrate(
        &mut self,
        message: Message,
        ctx: &mut Context<Ed>,
    ) -> Result<impl IntoIterator<Item = Message>, IntegrateError<Ed>> {
        match message {
            Message::CreatedCursor(cursor_creation) => {
                self.integrate_cursor_creation(cursor_creation, ctx);
                Ok(Messages::None)
            },

            Message::CreatedDirectory(directory_creation) => self
                .integrate_fs_op(directory_creation, ctx)
                .await
                .map(Messages::renames)
                .map_err(IntegrateError::FsOp),

            Message::CreatedFile(file_creation) => self
                .integrate_fs_op(file_creation, ctx)
                .await
                .map(Messages::renames)
                .map_err(IntegrateError::FsOp),

            Message::CreatedSelection(selection_creation) => {
                self.integrate_selection_creation(selection_creation, ctx);
                Ok(Messages::None)
            },

            Message::DeletedDirectory(deletion) => self
                .integrate_fs_op(deletion, ctx)
                .await
                .map(Messages::renames)
                .map_err(IntegrateError::FsOp),

            Message::DeletedFile(deletion) => self
                .integrate_fs_op(deletion, ctx)
                .await
                .map(Messages::renames)
                .map_err(IntegrateError::FsOp),

            Message::EditedBinary(binary_edit) => {
                self.integrate_binary_edit(binary_edit, ctx)
                    .await
                    .map_err(IntegrateError::BinaryEdit)?;
                Ok(Messages::None)
            },

            Message::EditedText(text_edit) => {
                self.integrate_text_edit(text_edit, ctx)
                    .await
                    .map_err(IntegrateError::CreateBuffer)?;
                Ok(Messages::None)
            },

            Message::MovedCursor(cursor_movement) => {
                self.integrate_cursor_move(cursor_movement, ctx);
                Ok(Messages::None)
            },

            Message::MovedDirectory(movement) => self
                .integrate_fs_op(movement, ctx)
                .await
                .map(Messages::renames)
                .map_err(IntegrateError::FsOp),

            Message::MovedFile(movement) => self
                .integrate_fs_op(movement, ctx)
                .await
                .map(Messages::renames)
                .map_err(IntegrateError::FsOp),

            Message::MovedSelection(selection_movement) => {
                self.integrate_selection_movement(selection_movement, ctx);
                Ok(Messages::None)
            },

            Message::PeerDisconnected(peer_id) => {
                self.integrate_peer_left(peer_id, ctx);
                Ok(Messages::None)
            },

            Message::PeerJoined(peer) => {
                self.integrate_peer_joined(peer, ctx);
                Ok(Messages::None)
            },

            Message::PeerLeft(peer_id) => {
                self.integrate_peer_left(peer_id, ctx);
                Ok(Messages::None)
            },

            Message::ProjectRequest(request) => {
                Ok(Messages::project_response(collab_types::ProjectResponse {
                    peers: self.peers(),
                    encoded_project: self.inner.encode(),
                    response_id: request.request_id.into(),
                }))
            },

            Message::ProjectResponse(_) => {
                tracing::error!(
                    title = %ctx.namespace().dot_separated(),
                    "received unexpected ProjectResponse"
                );
                Ok(Messages::None)
            },

            Message::RemovedCursor(cursor_deletion) => {
                self.integrate_cursor_deletion(cursor_deletion, ctx);
                Ok(Messages::None)
            },

            Message::RemovedSelection(selection_deletion) => {
                self.integrate_selection_deletion(selection_deletion, ctx);
                Ok(Messages::None)
            },

            Message::RenamedFsNode(rename) => self
                .integrate_fs_op(rename, ctx)
                .await
                .map(Messages::renames)
                .map_err(IntegrateError::FsOp),

            Message::SavedTextFile(file_id) => {
                self.integrate_file_save(file_id, ctx);
                Ok(Messages::None)
            },
        }
    }

    /// Returns the project's name.
    #[cfg(feature = "neovim")]
    pub(crate) fn name(&self) -> &abs_path::NodeName {
        self.root_path()
            .node_name()
            .expect("project can't be rooted at fs root")
    }

    /// Returns the project root's path.
    pub(crate) fn root_path(&self) -> &AbsPath {
        &self.root_path
    }

    /// TODO: docs.
    pub(crate) async fn synchronize(
        &mut self,
        event: Event<Ed>,
        ctx: &mut Context<Ed>,
    ) -> Result<Option<Message>, SynchronizeError<Ed>> {
        match event {
            Event::Buffer(event) => Ok(self.synchronize_buffer(event, ctx)),
            Event::Cursor(event) => Ok(Some(self.synchronize_cursor(event))),
            Event::Directory(event) => {
                self.synchronize_directory(event, ctx).await
            },
            Event::File(event) => self.synchronize_file(event, ctx).await,
            Event::Selection(event) => {
                Ok(Some(self.synchronize_selection(event)))
            },
        }
    }

    /// Returns the [`text::CursorMut`] corresponding to the cursor with the
    /// given ID.
    #[track_caller]
    fn cursor_of_cursor_id(
        &mut self,
        cursor_id: &Ed::CursorId,
    ) -> collab_project::text::CursorMut<'_> {
        let Some(&project_cursor_id) =
            self.id_maps.cursor2cursor.get(cursor_id)
        else {
            panic!("unknown cursor ID: {cursor_id:?}");
        };

        let Ok(maybe_cursor) = self.inner.cursor_mut(project_cursor_id) else {
            panic!("cursor ID {cursor_id:?} maps to a remote peer's cursor")
        };

        match maybe_cursor {
            Some(cursor) => cursor,
            None => {
                panic!("cursor ID {cursor_id:?} maps to a deleted cursor")
            },
        }
    }

    async fn integrate_binary_edit(
        &mut self,
        edit: binary::BinaryEdit,
        ctx: &mut Context<Ed>,
    ) -> Result<(), IntegrateBinaryEditError<Ed::Fs>> {
        let Some(file_mut) = self.inner.integrate_binary_edit(edit) else {
            return Ok(());
        };

        let file = file_mut.as_file();
        let file_path = self.root_path.clone().concat(file.path());
        let new_contents = file.contents().to_owned();

        let fs = ctx.fs();

        ctx.spawn_background(async move {
            let Some(node) = fs
                .node_at_path(&*file_path)
                .await
                .map_err(IntegrateBinaryEditError::NodeAtPath)?
            else {
                return Err(IntegrateBinaryEditError::NoNodeAtPath(file_path));
            };

            let mut file = match node {
                fs::Node::File(file) => file,
                fs::Node::Directory(_) => {
                    return Err(IntegrateBinaryEditError::DirectoryAtPath(
                        file_path,
                    ));
                },
                fs::Node::Symlink(_) => {
                    return Err(IntegrateBinaryEditError::SymlinkAtPath(
                        file_path,
                    ));
                },
            };

            file.write(new_contents)
                .await
                .map_err(IntegrateBinaryEditError::WriteToFile)
        })
        .await
    }

    /// Integrates the creation of a remote cursor, creating a tooltip in the
    /// corresponding buffer (if it's currently open).
    pub fn integrate_cursor_creation(
        &mut self,
        creation: text::CursorCreation,
        ctx: &mut Context<Ed>,
    ) {
        let mut try_block = || {
            let cursor = self.inner.integrate_cursor_creation(creation)?;

            let cursor_owner = self.remote_peers.with_mut(|map| {
                let peer = map.get_mut(&cursor.owner())?;

                // Check if the new cursor should become the peer's main
                // cursor.
                if peer.main_cursor_id().is_none_or(|id| cursor.id() < id) {
                    peer.set_main_cursor_id(cursor.id());
                }

                Some(peer.clone())
            })?;

            let buffer_id =
                self.id_maps.file2buffer.get(&cursor.file().local_id())?;

            let tooltip = Ed::create_peer_tooltip(
                cursor_owner.into(),
                cursor.offset(),
                buffer_id.clone(),
                ctx,
            );

            self.peer_tooltips.insert(cursor.id(), tooltip);

            Some(())
        };

        try_block();
    }

    /// Integrates the deletion of a remote cursor.
    pub fn integrate_cursor_deletion(
        &mut self,
        removal: text::CursorRemoval,
        ctx: &mut Context<Ed>,
    ) {
        let mut try_block = || {
            let cursor_id = self.inner.integrate_cursor_removal(removal)?;

            // Remove any UI element associated with this cursor.
            if let Some(tooltip) = self.peer_tooltips.remove(&cursor_id) {
                Ed::remove_peer_tooltip(tooltip, ctx);
            }

            // Check if this was a peer's main cursor.
            self.remote_peers.with_mut(|map| {
                if let Some(peer) = map.get_mut(&cursor_id.owner())
                    && peer.main_cursor_id() == Some(cursor_id)
                {
                    peer.remove_main_cursor();
                }
            });

            Some(())
        };

        try_block();
    }

    fn integrate_cursor_move(
        &mut self,
        movement: text::CursorMove,
        ctx: &mut Context<Ed>,
    ) {
        let mut try_block = || {
            let cursor = self.inner.integrate_cursor_move(movement)?;
            let tooltip = self.peer_tooltips.get_mut(&cursor.id())?;
            Ed::move_peer_tooltip(tooltip, cursor.offset(), ctx);
            Some(())
        };

        try_block();
    }

    fn integrate_file_save(
        &self,
        global_id: GlobalFileId,
        ctx: &mut Context<Ed>,
    ) {
        let try_block = || {
            let file_id = self.inner.local_file_of_global(global_id)?;
            let buffer_id = self.id_maps.file2buffer.get(&file_id)?;
            ctx.with_borrowed(|ctx| {
                let Some(mut buffer) = ctx.buffer(buffer_id.clone()) else {
                    panic!("{buffer_id:?} doesn't exist");
                };
                if Ed::should_remote_save_cause_local_save(&buffer) {
                    let _ = buffer.schedule_save(self.agent_id);
                }
            });
            Some(())
        };

        try_block();
    }

    /// TODO: docs.
    async fn integrate_fs_op<T: FsOp>(
        &mut self,
        op: T,
        ctx: &mut Context<Ed>,
    ) -> Result<SmallVec<[Rename; 2]>, IntegrateFsOpError<Ed::Fs>> {
        let mut actions = SmallVec::new();
        let mut renames = SmallVec::new();
        let peers = self.map_peers(|peer| (peer.id, peer.clone()));

        let mut sync_actions = self.inner.integrate_fs_op(op);

        while let Some(sync_action) = sync_actions.next() {
            if let Some(more_renames) =
                impl_integrate_fs_op::push_resolved_actions(
                    sync_action,
                    &peers,
                    &mut actions,
                )
            {
                renames.extend(more_renames);
            }
        }

        let fs = ctx.fs();

        ctx.spawn_background(async move {
            for action in actions {
                action.apply(&fs).await?;
            }
            Ok(())
        })
        .await?;

        Ok(renames)
    }

    fn integrate_peer_joined(&self, peer: Peer, ctx: &mut Context<Ed>) {
        Ed::on_peer_joined(&peer, self, ctx);
        self.remote_peers.insert(peer, &self.inner);
    }

    fn integrate_peer_left(&mut self, peer_id: PeerId, ctx: &mut Context<Ed>) {
        let peer = self.remote_peers.remove(peer_id);

        let (cursor_ids, selection_ids) =
            self.inner.integrate_peer_disconnection(peer_id);

        for cursor_id in cursor_ids {
            if let Some(tooltip) = self.peer_tooltips.remove(&cursor_id) {
                Ed::remove_peer_tooltip(tooltip, ctx);
            }
        }

        for selection_id in selection_ids {
            if let Some(selection) = self.peer_selections.remove(&selection_id)
            {
                Ed::remove_peer_selection(selection, ctx);
            }
        }

        Ed::on_peer_left(&peer, self, ctx);
    }

    fn integrate_selection_creation(
        &mut self,
        creation: text::SelectionCreation,
        ctx: &mut Context<Ed>,
    ) {
        let mut try_block = || {
            let selection =
                self.inner.integrate_selection_creation(creation)?;
            let file_id = selection.file()?.local_id();
            let buffer_id = self.id_maps.file2buffer.get(&file_id)?;
            let selection_owner = self.remote_peers.get(selection.owner())?;
            let peer_selection = Ed::create_peer_selection(
                selection_owner.into(),
                selection.offset_range(),
                buffer_id.clone(),
                ctx,
            );
            self.peer_selections.insert(selection.id(), peer_selection);
            Some(())
        };

        try_block();
    }

    fn integrate_selection_deletion(
        &mut self,
        deletion: text::SelectionRemoval,
        ctx: &mut Context<Ed>,
    ) {
        let mut try_block = || {
            let sel_id = self.inner.integrate_selection_removal(deletion)?;
            let peer_selection = self.peer_selections.remove(&sel_id)?;
            Ed::remove_peer_selection(peer_selection, ctx);
            Some(())
        };

        try_block();
    }

    fn integrate_selection_movement(
        &mut self,
        movement: text::SelectionMove,
        ctx: &mut Context<Ed>,
    ) {
        let mut try_block = || {
            let selection = self.inner.integrate_selection_move(movement)?;
            let peer_selection =
                self.peer_selections.get_mut(&selection.id())?;
            Ed::move_peer_selection(
                peer_selection,
                selection.offset_range(),
                ctx,
            );
            Some(())
        };

        try_block();
    }

    /// Integrates a remote text edit by applying it to the corresponding
    /// buffer, creating the buffer first if necessary.
    ///
    /// Returns the ID of the buffer the edit was applied to, or `None` if
    /// integrating the edit didn't generate any replacements to be applied.
    pub async fn integrate_text_edit(
        &mut self,
        edit: text::TextEdit,
        ctx: &mut Context<Ed>,
    ) -> Result<Option<Ed::BufferId>, Ed::CreateBufferError> {
        let Some((file, replacements)) = self.inner.integrate_text_edit(edit)
        else {
            return Ok(None);
        };

        let file_id = file.local_id();

        // If there's already an open buffer for the edited file we can just
        // apply the replacements to it. If not, we have to first create one.
        let buffer_id = match self.id_maps.file2buffer.get(&file_id) {
            Some(buffer_id) => buffer_id.clone(),
            None => {
                let file_path = self.root_path.clone().concat(file.path());
                let buffer_id =
                    ctx.create_buffer(&file_path, self.agent_id).await?;
                self.synchronize_buffer_created(
                    buffer_id.clone(),
                    &file_path,
                    ctx,
                );
                buffer_id
            },
        };

        ctx.with_borrowed(|ctx| {
            ctx.buffer(buffer_id.clone())
                .expect("buffer exists")
                .schedule_edit(
                    replacements.into_iter().map(Convert::convert),
                    self.agent_id,
                )
                .boxed_local()
        })
        .await;

        let Some(File::Text(file)) = self.inner.file(file_id) else {
            unreachable!("we know this ID maps to a text file");
        };

        Self::update_cursors(&file, &mut self.peer_tooltips, ctx);
        Self::update_selections(&file, &mut self.peer_selections, ctx);

        Ok(Some(buffer_id))
    }

    fn map_peers<T, Collector: FromIterator<T>>(
        &self,
        fun: impl FnMut(&Peer) -> T,
    ) -> Collector {
        self.remote_peers.with(|remote_peers| {
            remote_peers
                .values()
                .map(core::ops::Deref::deref)
                .chain(iter::once(&self.local_peer))
                .map(fun)
                .collect()
        })
    }

    fn peers<Collector: FromIterator<Peer>>(&self) -> Collector {
        self.map_peers(Clone::clone)
    }

    /// Returns the [`NodeMut`] corresponding to the node with the given
    /// ID.
    #[track_caller]
    fn project_node(
        &mut self,
        node_id: &<Ed::Fs as fs::Fs>::NodeId,
    ) -> NodeMut<'_> {
        if let Some(&dir_id) = self.id_maps.node2dir.get(node_id) {
            let Some(dir) = self.inner.directory_mut(dir_id) else {
                panic!("node ID {node_id:?} maps to a deleted directory")
            };
            NodeMut::Directory(dir)
        } else if let Some(&file_id) = self.id_maps.node2file.get(node_id) {
            let Some(file) = self.inner.file_mut(file_id) else {
                panic!("node ID {node_id:?} maps to a deleted file")
            };
            NodeMut::File(file)
        } else {
            panic!("unknown node ID: {node_id:?}")
        }
    }

    /// Returns the [`text::SelectionMut`] corresponding to the selection with
    /// the given ID.
    #[track_caller]
    fn selection_of_selection_id(
        &mut self,
        selection_id: &Ed::SelectionId,
    ) -> collab_project::text::SelectionMut<'_> {
        let Some(&project_selection_id) =
            self.id_maps.selection2selection.get(selection_id)
        else {
            panic!("unknown selection ID: {selection_id:?}");
        };

        let Ok(maybe_selection) =
            self.inner.selection_mut(project_selection_id)
        else {
            panic!(
                "selection ID {selection_id:?} maps to a remote peer's \
                 selection"
            )
        };

        match maybe_selection {
            Some(selection) => selection,
            None => {
                panic!(
                    "selection ID {selection_id:?} maps to a deleted \
                     selection"
                )
            },
        }
    }

    /// Synchronizes the project's state with the given buffer event.
    pub fn synchronize_buffer(
        &mut self,
        event: event::BufferEvent<Ed>,
        ctx: &mut Context<Ed>,
    ) -> Option<Message> {
        match event {
            event::BufferEvent::Created(buffer_id, file_path) => {
                self.synchronize_buffer_created(buffer_id, &file_path, ctx);
                None
            },
            event::BufferEvent::Edited(buffer_id, replacements) => {
                self.synchronize_buffer_edited(buffer_id, replacements, ctx)
            },
            event::BufferEvent::Removed(buffer_id) => {
                let ids = &mut self.id_maps;
                if let Some(file_id) = ids.buffer2file.remove(&buffer_id) {
                    ids.file2buffer.remove(&file_id);
                }
                None
            },
            event::BufferEvent::Saved(buffer_id) => {
                let file_id = self.text_file_of_buffer(&buffer_id).global_id();
                Some(Message::SavedTextFile(file_id))
            },
        }
    }

    /// Synchronizes the project's state with the creation of a new buffer
    /// backed by the file at the given path.
    pub fn synchronize_buffer_created(
        &mut self,
        buffer_id: Ed::BufferId,
        file_path: &AbsPath,
        ctx: &mut Context<Ed>,
    ) {
        let path_in_proj = file_path
            .strip_prefix(&self.root_path)
            .expect("the buffer is backed by a file in the project");

        let Some(Node::File(File::Text(file))) =
            self.inner.node_at_path(path_in_proj)
        else {
            return;
        };

        // Record the mapping between the buffer ID and the file ID for O(1)
        // lookups in both directions.
        self.id_maps.buffer2file.insert(buffer_id.clone(), file.local_id());
        self.id_maps.file2buffer.insert(file.local_id(), buffer_id.clone());

        // Display the cursors of the remote peers in the buffer.
        for cursor in file.cursors() {
            let Some(owner) = self.remote_peers.get(cursor.owner()) else {
                continue;
            };
            let tooltip = Ed::create_peer_tooltip(
                owner.into(),
                cursor.offset(),
                buffer_id.clone(),
                ctx,
            );
            self.peer_tooltips.insert(cursor.id(), tooltip);
        }

        // Display the selections of the remote peers in the buffer.
        for selection in file.selections() {
            let Some(owner) = self.remote_peers.get(selection.owner()) else {
                continue;
            };
            let peer_selection = Ed::create_peer_selection(
                owner.into(),
                selection.offset_range(),
                buffer_id.clone(),
                ctx,
            );
            self.peer_selections.insert(selection.id(), peer_selection);
        }
    }

    fn synchronize_buffer_edited(
        &mut self,
        buffer_id: Ed::BufferId,
        replacements: impl IntoIterator<Item = editor::Replacement>,
        ctx: &mut Context<Ed>,
    ) -> Option<Message> {
        let mut file_mut = self.text_file_of_buffer(&buffer_id);

        let text_edit =
            file_mut.edit(replacements.into_iter().map(Convert::convert));

        let file_id = file_mut.local_id();

        let Some(File::Text(file)) = self.inner.file(file_id) else {
            unreachable!("we know this ID maps to a text file");
        };

        Self::update_cursors(&file, &mut self.peer_tooltips, ctx);
        Self::update_selections(&file, &mut self.peer_selections, ctx);

        Some(Message::EditedText(text_edit))
    }

    fn synchronize_cursor(
        &mut self,
        event: event::CursorEvent<Ed>,
    ) -> Message {
        match event.kind {
            event::CursorEventKind::Created(buffer_id, byte_offset) => {
                let (cursor_id, creation) = self
                    .text_file_of_buffer(&buffer_id)
                    .create_cursor(byte_offset);

                self.id_maps.cursor2cursor.insert(event.cursor_id, cursor_id);

                Message::CreatedCursor(creation)
            },
            event::CursorEventKind::Moved(byte_offset) => {
                let movement = self
                    .cursor_of_cursor_id(&event.cursor_id)
                    .r#move(byte_offset);

                Message::MovedCursor(movement)
            },
            event::CursorEventKind::Removed => {
                let deletion =
                    self.cursor_of_cursor_id(&event.cursor_id).delete();

                self.id_maps.cursor2cursor.remove(&event.cursor_id);

                Message::RemovedCursor(deletion)
            },
        }
    }

    async fn synchronize_directory(
        &mut self,
        event: fs::DirectoryEvent<Ed::Fs>,
        ctx: &mut Context<Ed>,
    ) -> Result<Option<Message>, SynchronizeError<Ed>> {
        match event {
            fs::DirectoryEvent::Creation(creation) => {
                self.synchronize_node_creation(creation, ctx).await
            },
            fs::DirectoryEvent::Deletion(deletion) => {
                Ok(Some(self.synchronize_node_deletion(deletion)))
            },
            fs::DirectoryEvent::Move(r#move) => {
                Ok(Some(self.synchronize_node_move(r#move)))
            },
        }
    }

    async fn synchronize_file(
        &mut self,
        event: fs::FileEvent<Ed::Fs>,
        ctx: &mut Context<Ed>,
    ) -> Result<Option<Message>, SynchronizeError<Ed>> {
        match event {
            fs::FileEvent::Modification(modification) => {
                self.synchronize_file_modification(modification, ctx).await
            },
            fs::FileEvent::IdChange(id_change) => {
                self.synchronize_file_id_change(id_change);
                Ok(None)
            },
        }
    }

    fn synchronize_file_id_change(
        &mut self,
        id_change: fs::FileIdChange<Ed::Fs>,
    ) {
        match self.id_maps.node2file.remove(&id_change.old_id) {
            Some(file_id) => {
                self.id_maps.node2file.insert(id_change.new_id, file_id);
            },
            None => {
                panic!("unknown node ID: {:?}", id_change.old_id);
            },
        }
    }

    async fn synchronize_file_modification(
        &mut self,
        modification: fs::FileModification<Ed::Fs>,
        ctx: &mut Context<Ed>,
    ) -> Result<Option<Message>, SynchronizeError<Ed>> {
        enum FileContents {
            Binary(Arc<[u8]>),
            Text(crop::Rope),
        }

        enum FileDiff {
            Binary(Vec<u8>),
            Text(SmallVec<[TextReplacement; 1]>),
        }

        let root_path = self.root_path.clone();

        let file_mut = match self.project_node(&modification.file_id) {
            NodeMut::File(file) => file,
            NodeMut::Directory(_) => {
                panic!("received a FileModification event on a directory")
            },
        };

        let file_path = root_path.concat(file_mut.path());

        // Get the file's contents before the modification.
        let file_contents = match file_mut.as_file() {
            File::Binary(file) => FileContents::Binary(file.contents().into()),
            File::Text(file) => FileContents::Text(file.contents().clone()),
            File::Symlink(_) => {
                panic!("received a FileModification event on a symlink")
            },
        };

        let fs = ctx.fs();

        // Compute a diff with the current file contents in the background.
        let compute_diff = ctx.spawn_background(async move {
            let Some(node_contents) = fs.contents_at_path(&file_path).await?
            else {
                return Ok(None);
            };

            Ok(match (file_contents, node_contents) {
                (FileContents::Binary(lhs), FsNodeContents::Binary(rhs)) => {
                    (*lhs != *rhs).then_some(FileDiff::Binary(rhs))
                },
                (FileContents::Text(lhs), FsNodeContents::Text(rhs)) => {
                    text_diff(lhs, &rhs).map(FileDiff::Text)
                },
                _ => None,
            })
        });

        let file_diff = match compute_diff.await {
            Ok(Some(file_diff)) => file_diff,
            Ok(None) => return Ok(None),
            Err(err) => return Err(SynchronizeError::ContentsAtPath(err)),
        };

        // Apply the diff.
        Ok(Some(match (file_mut, file_diff) {
            (FileMut::Binary(mut file), FileDiff::Binary(contents)) => {
                Message::EditedBinary(file.replace(contents))
            },
            (FileMut::Text(mut file), FileDiff::Text(replacements)) => {
                Message::EditedText(file.edit(replacements))
            },
            _ => unreachable!(),
        }))
    }

    async fn synchronize_node_creation(
        &mut self,
        creation: fs::NodeCreation<Ed::Fs>,
        ctx: &mut Context<Ed>,
    ) -> Result<Option<Message>, SynchronizeError<Ed>> {
        let node_contents =
            match ctx.fs().contents_at_path(&creation.node_path).await {
                Ok(Some(contents)) => contents,

                // The node must've already been deleted or moved.
                //
                // FIXME: doing nothing can be problematic if we're about to
                // receive deletions/moves for the node.
                Ok(None) => return Ok(None),

                Err(err) => return Err(SynchronizeError::ContentsAtPath(err)),
            };

        let node_id = creation.node_id;

        let node_path = creation.node_path;

        let mut components = node_path.components();

        let node_name =
            components.next_back().expect("root can't be created").to_owned();

        let parent_path = components.as_path();

        let parent_path_in_project = parent_path
            .strip_prefix(&self.root_path)
            .expect("the new parent has to be in the project");

        let Some(parent) = self.inner.node_at_path_mut(parent_path_in_project)
        else {
            panic!(
                "parent path {parent_path_in_project:?} doesn't exist in the \
                 project"
            );
        };

        let NodeMut::Directory(mut parent) = parent else {
            panic!("parent is not a directory");
        };

        let Ok((creation, file_mut)) = (match node_contents {
            FsNodeContents::Directory => {
                match parent.create_directory(node_name) {
                    Ok((creation, dir_mut)) => {
                        let dir_id = dir_mut.as_directory().id();
                        self.id_maps.node2dir.insert(node_id, dir_id);
                        return Ok(Some(Message::CreatedDirectory(creation)));
                    },
                    Err(err) => Err(err),
                }
            },
            FsNodeContents::Text(text_contents) => {
                parent.create_text_file(node_name, text_contents)
            },
            FsNodeContents::Binary(binary_contents) => {
                parent.create_binary_file(node_name, binary_contents)
            },
            FsNodeContents::Symlink(target_path) => {
                parent.create_symlink(node_name, target_path)
            },
        }) else {
            unreachable!("no duplicate node names");
        };

        let file_id = file_mut.as_file().id();
        self.id_maps.node2file.insert(node_id, file_id);
        Ok(Some(Message::CreatedFile(creation)))
    }

    fn synchronize_node_deletion(
        &mut self,
        deletion: fs::NodeDeletion<Ed::Fs>,
    ) -> Message {
        let node_id = deletion.node_id;

        let deletion = match self.project_node(&node_id) {
            NodeMut::Directory(dir) => match dir.delete() {
                Ok(deletion) => Message::DeletedDirectory(deletion),
                Err(_) => unreachable!("dir is not the project root"),
            },
            NodeMut::File(file) => Message::DeletedFile(file.delete()),
        };

        let ids = &mut self.id_maps;
        if let Some(file_id) = ids.node2file.remove(&node_id) {
            if let Some(buffer_id) = ids.file2buffer.remove(&file_id) {
                ids.buffer2file.remove(&buffer_id);
            }
        } else {
            ids.node2dir.remove(&node_id);
        }

        deletion
    }

    fn synchronize_node_move(
        &mut self,
        r#move: fs::NodeMove<Ed::Fs>,
    ) -> Message {
        let parent_path =
            r#move.new_path.parent().expect("root can't be moved");

        let parent_path_in_project = parent_path
            .strip_prefix(&self.root_path)
            .expect("the new parent has to be in the project");

        let Some(parent) = self.inner.node_at_path_mut(parent_path_in_project)
        else {
            panic!(
                "parent path {parent_path_in_project:?} doesn't exist in the \
                 project"
            );
        };

        let NodeMut::Directory(parent) = parent else {
            panic!("parent is not a directory");
        };

        let parent_id = parent.as_directory().id();

        match self.project_node(&r#move.node_id) {
            NodeMut::Directory(mut dir) => Message::MovedDirectory(
                dir.r#move(parent_id).expect("invalid move on directory"),
            ),

            NodeMut::File(mut file) => Message::MovedFile(
                file.r#move(parent_id).expect("invalid move on file"),
            ),
        }
    }

    fn synchronize_selection(
        &mut self,
        event: event::SelectionEvent<Ed>,
    ) -> Message {
        match event.kind {
            event::SelectionEventKind::Created(buffer_id, byte_range) => {
                let (selection_id, creation) = self
                    .text_file_of_buffer(&buffer_id)
                    .create_selection(byte_range);

                self.id_maps
                    .selection2selection
                    .insert(event.selection_id, selection_id);

                Message::CreatedSelection(creation)
            },
            event::SelectionEventKind::Moved(byte_range) => {
                let movement = self
                    .selection_of_selection_id(&event.selection_id)
                    .r#move(byte_range);

                Message::MovedSelection(movement)
            },
            event::SelectionEventKind::Removed => {
                let removal = self
                    .selection_of_selection_id(&event.selection_id)
                    .delete();

                self.id_maps.selection2selection.remove(&event.selection_id);

                Message::RemovedSelection(removal)
            },
        }
    }

    /// Returns the [`text::TextFileMut`] corresponding to the buffer with the
    /// given ID.
    #[track_caller]
    fn text_file_of_buffer(
        &mut self,
        buffer_id: &Ed::BufferId,
    ) -> collab_project::text::TextFileMut<'_> {
        let Some(&file_id) = self.id_maps.buffer2file.get(buffer_id) else {
            panic!("unknown buffer ID: {buffer_id:?}");
        };

        let Some(file) = self.inner.file_mut(file_id) else {
            panic!("buffer ID {buffer_id:?} maps to a deleted file")
        };

        match file {
            FileMut::Text(text_file) => text_file,
            FileMut::Binary(_) => {
                panic!("buffer ID {buffer_id:?} maps to a binary file")
            },
            FileMut::Symlink(_) => {
                panic!("buffer ID {buffer_id:?} maps to a symlink file")
            },
        }
    }

    /// Updates the positions of all the remote peers' cursors in the given
    /// file.
    fn update_cursors(
        file: &collab_project::text::TextFile,
        peer_cursors: &mut FxHashMap<CursorId, Ed::PeerTooltip>,
        ctx: &mut Context<Ed>,
    ) {
        for cursor in file.cursors() {
            let Some(editor_cursor) = peer_cursors.get_mut(&cursor.id())
            else {
                // This cursor must be owned by the local peer.
                continue;
            };

            Ed::move_peer_tooltip(editor_cursor, cursor.offset(), ctx);
        }
    }

    /// Updates the positions of all the remote peers' selections in the given
    /// file.
    fn update_selections(
        file: &collab_project::text::TextFile,
        peer_selections: &mut FxHashMap<SelectionId, Ed::PeerSelection>,
        ctx: &mut Context<Ed>,
    ) {
        for selection in file.selections() {
            let Some(editor_selection) =
                peer_selections.get_mut(&selection.id())
            else {
                // This selection must be owned by the local peer.
                continue;
            };

            Ed::move_peer_selection(
                editor_selection,
                selection.offset_range(),
                ctx,
            );
        }
    }
}

mod impl_integrate_fs_op {
    //! Contains the various types, free-standing functions and methods used in
    //! the implementation of [`ProjectHandle::integrate_fs_op`].

    use abs_path::{NodeName, NodeNameBuf};
    use collab_project::fs::{ResolveConflict, SyncAction};
    use compact_str::format_compact;
    use fs::Directory;
    use futures_util::FutureExt;
    use puff::node::IsVisible;

    use super::*;

    pub(super) enum FileContents {
        Binary(Arc<[u8]>),
        Text(crop::Rope),
        Symlink(compact_str::CompactString),
    }

    /// The type of file system operation that should be performed as a
    /// result of
    pub(super) enum ResolvedFsAction {
        /// Create a directory at the given path.
        CreateDirectory(AbsPathBuf),
        /// Create a file at the given path with the given contents.
        CreateFile(AbsPathBuf, FileContents),
        /// Delete the node at the given path.
        DeleteNode(AbsPathBuf),
        /// Move a node from the first path to the second path.
        MoveNode(AbsPathBuf, AbsPathBuf),
    }

    /// The type of action that caused a naming conflict between two file
    /// system nodes that are both under the same directory.
    enum NamingConflictSource {
        Creation,
        Movement,
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn push_resolved_actions(
        action: SyncAction<'_>,
        peers: &FxHashMap<PeerId, Peer>,
        actions: &mut SmallVec<[ResolvedFsAction; 1]>,
    ) -> Option<[Rename; 2]> {
        match action {
            SyncAction::Create(create) => {
                push_node_creation(create.node(), actions);
                None
            },
            SyncAction::Delete(delete) => {
                actions.push(ResolvedFsAction::DeleteNode(delete.old_path()));
                None
            },
            SyncAction::Move(r#move) => {
                actions.push(ResolvedFsAction::MoveNode(
                    r#move.old_path(),
                    r#move.new_path(),
                ));
                None
            },
            SyncAction::Rename(rename) => {
                actions.push(ResolvedFsAction::MoveNode(
                    rename.old_path(),
                    rename.new_path(),
                ));
                None
            },
            SyncAction::CreateAndResolve(mut create_and_resolve) => {
                let create = create_and_resolve.create();
                let create_node = create.node();
                let create_node_path = create_node.path();
                let orig_len = actions.len();

                // Push a move action for the existing node causing the
                // conflict. We'll replace the destination path once we've
                // resolved the conflict.
                actions.push(ResolvedFsAction::MoveNode(
                    create_node_path.clone(),
                    create_node_path,
                ));

                // Push the creation actions for the new node.
                push_node_creation(create_node, actions);

                let (rename_conflicting, rename_existing) =
                    resolve_naming_conflict(
                        create_and_resolve.into_resolve(),
                        NamingConflictSource::Creation,
                        peers,
                    );

                match &mut actions[orig_len] {
                    ResolvedFsAction::MoveNode(_, dest_path) => {
                        dest_path.pop();
                        dest_path.push(rename_existing.new_name());
                    },
                    _ => unreachable!("we pushed a MoveNode action above"),
                }

                match &mut actions[orig_len + 1] {
                    ResolvedFsAction::CreateDirectory(path)
                    | ResolvedFsAction::CreateFile(path, _) => {
                        path.pop();
                        path.push(rename_conflicting.new_name());
                    },
                    _ => unreachable!("we pushed a Create* action above"),
                }

                Some([rename_conflicting, rename_existing])
            },
            SyncAction::MoveAndResolve(mut move_and_resolve) => {
                let r#move = move_and_resolve.r#move();
                Some(push_move_and_resolve(
                    r#move.new_path(),
                    r#move.old_path(),
                    move_and_resolve.into_resolve(),
                    peers,
                    actions,
                ))
            },
            SyncAction::RenameAndResolve(mut rename_and_resolve) => {
                let rename = rename_and_resolve.rename();
                Some(push_move_and_resolve(
                    rename.new_path(),
                    rename.old_path(),
                    rename_and_resolve.into_resolve(),
                    peers,
                    actions,
                ))
            },
        }
    }

    /// Pushes the actions corresponding to the creation of the given node into
    /// the given buffer.
    ///
    /// If the node is a directory, it recursively pushes the creation actions
    /// for all its children.
    fn push_node_creation(
        node: Node<impl IsVisible>,
        actions: &mut SmallVec<[ResolvedFsAction; 1]>,
    ) {
        match node {
            Node::Directory(dir) => {
                actions.push(ResolvedFsAction::CreateDirectory(dir.path()));
                for child in dir.children() {
                    push_node_creation(child, actions);
                }
            },
            Node::File(file) => {
                let file_contents = match file {
                    File::Binary(binary) => {
                        FileContents::Binary(binary.contents().into())
                    },
                    File::Symlink(symlink) => {
                        FileContents::Symlink(symlink.target_path().into())
                    },
                    File::Text(text) => {
                        FileContents::Text(text.contents().clone())
                    },
                };
                actions.push(ResolvedFsAction::CreateFile(
                    file.path(),
                    file_contents,
                ));
            },
        }
    }

    /// TODO: docs.
    #[allow(clippy::too_many_arguments)]
    fn push_move_and_resolve(
        move_existing_from: AbsPathBuf,
        move_conflicting_from: AbsPathBuf,
        conflict: ResolveConflict<'_>,
        peers: &FxHashMap<PeerId, Peer>,
        actions: &mut SmallVec<[ResolvedFsAction; 1]>,
    ) -> [Rename; 2] {
        let (rename_conflicting, rename_existing) = resolve_naming_conflict(
            conflict,
            NamingConflictSource::Movement,
            peers,
        );

        let move_existing_to = {
            let mut path = move_existing_from.clone();
            path.pop();
            path.push(rename_existing.new_name());
            path
        };

        let move_conflicting_to = {
            let mut path = move_existing_from.clone();
            path.pop();
            path.push(rename_conflicting.new_name());
            path
        };

        actions.push(ResolvedFsAction::MoveNode(
            move_existing_from,
            move_existing_to,
        ));

        actions.push(ResolvedFsAction::MoveNode(
            move_conflicting_from,
            move_conflicting_to,
        ));

        [rename_conflicting, rename_existing]
    }

    #[allow(clippy::too_many_lines)]
    fn resolve_naming_conflict(
        mut conflict: ResolveConflict<'_>,
        conflict_source: NamingConflictSource,
        peers: &FxHashMap<PeerId, Peer>,
    ) -> (Rename, Rename) {
        let conflicting = conflict.conflicting_node();
        let existing = conflict.existing_node();

        // If the naming conflict is due to concurrent creations, we'll first
        // try to resolve it by appending the GitHub handles of the creators to
        // the file names.
        //
        // For example, if Alice and Bob concurrently create a "lib.rs" file in
        // the same directory, we'll rename them to "lib.rs-alice" and
        // "lib.rs-bob", respectively.
        //
        // In the rare edge case where doing that doesn't break the conflict
        // (for example if a file named "lib.rs-alice" already exists), we'll
        // fallback to the logic below, which will append random suffixes to
        // the new names.
        if let NamingConflictSource::Creation = conflict_source {
            debug_assert!(
                conflicting.created_by() != existing.created_by(),
                "conflicting and existing nodes must have different creators"
            );

            if let (Some(creator_conflicting), Some(creator_existing)) = (
                peers.get(&conflicting.created_by()),
                peers.get(&existing.created_by()),
            ) {
                let gen_name =
                    |current_name: &NodeName, node_creator: &Peer| {
                        let suffix = node_creator.handle.as_str();
                        format_compact!("{current_name}-{suffix}")
                            .parse::<NodeNameBuf>()
                            .expect("new name is valid")
                    };

                let mut conflicting = conflict.conflicting_node_mut();
                let new_name = gen_name(
                    conflicting.try_name().expect("node is not root"),
                    creator_conflicting,
                );
                let rename_conflicting = conflicting.force_rename(new_name);

                let mut existing = conflict.existing_node_mut();
                let new_name = gen_name(
                    existing.try_name().expect("node is not root"),
                    creator_existing,
                );
                let rename_existing = existing.force_rename(new_name);

                match conflict.assume_resolved() {
                    Ok(()) => return (rename_conflicting, rename_existing),
                    Err(still_conflict) => conflict = still_conflict,
                }
            }
        }

        let conflicting = conflict.conflicting_node();
        let existing = conflict.existing_node();

        // Create 2 deterministically-seeded RNGs to produce name suffixes.
        let (seed_conflicting, seed_existing) =
            if conflicting.created_by() != existing.created_by() {
                (conflicting.created_by().into(), existing.created_by().into())
            } else {
                let seed = existing.created_by().into();
                let mut rng = fastrand::Rng::with_seed(seed);
                (rng.u64(..), rng.u64(..))
            };

        debug_assert!(seed_conflicting != seed_existing);

        let mut rng_conflicting = fastrand::Rng::with_seed(seed_conflicting);
        let mut rng_existing = fastrand::Rng::with_seed(seed_existing);

        let gen_name = |current_name: &NodeName, rng: &mut fastrand::Rng| {
            let suffix = iter::repeat_with(|| rng.alphanumeric())
                .take(6)
                .map(|ch| ch.to_ascii_lowercase())
                .collect::<compact_str::CompactString>();
            format_compact!("{current_name}-{suffix}")
                .parse::<NodeNameBuf>()
                .expect("new name is valid")
        };

        let orig_name_conflicting =
            conflicting.try_name().expect("node is not root").to_owned();

        let orig_name_existing =
            existing.try_name().expect("node is not root").to_owned();

        loop {
            let mut conflicting = conflict.conflicting_node_mut();
            let new_name =
                gen_name(&orig_name_conflicting, &mut rng_conflicting);
            let rename_conflicting = conflicting.force_rename(new_name);

            let mut existing = conflict.existing_node_mut();
            let new_name = gen_name(&orig_name_existing, &mut rng_existing);
            let rename_existing = existing.force_rename(new_name);

            match conflict.assume_resolved() {
                Ok(()) => return (rename_conflicting, rename_existing),
                Err(still_conflict) => conflict = still_conflict,
            }
        }
    }

    impl ResolvedFsAction {
        pub(super) async fn apply<Fs: fs::Fs>(
            self,
            fs: &Fs,
        ) -> Result<(), IntegrateFsOpError<Fs>> {
            match self {
                Self::CreateDirectory(path) => {
                    let (parent_path, dir_name) =
                        path.split_last().expect("not creating root");

                    fs.dir(parent_path)
                        .await
                        .map_err(IntegrateFsOpError::GetDir)?
                        .create_directory(dir_name)
                        .await
                        .map(|_| ())
                        .map_err(IntegrateFsOpError::CreateDirectory)
                },
                Self::CreateFile(path, contents) => {
                    let (parent_path, file_name) =
                        path.split_last().expect("not creating root");

                    let parent = fs
                        .dir(parent_path)
                        .await
                        .map_err(IntegrateFsOpError::GetDir)?;

                    match contents {
                        FileContents::Binary(contents) => parent
                            .create_file(file_name)
                            .await
                            .map_err(IntegrateFsOpError::CreateFile)?
                            .write(contents)
                            .await
                            .map_err(IntegrateFsOpError::WriteFile),

                        FileContents::Symlink(target_path) => parent
                            .create_symlink(file_name, &target_path)
                            .await
                            .map(|_| ())
                            .map_err(IntegrateFsOpError::CreateSymlink),

                        FileContents::Text(rope) => parent
                            .create_file(file_name)
                            .await
                            .map_err(IntegrateFsOpError::CreateFile)?
                            .write_chunks(rope.chunks())
                            .boxed()
                            .await
                            .map_err(IntegrateFsOpError::WriteFile),
                    }
                },
                Self::DeleteNode(path) => fs
                    .delete_node(&path)
                    .await
                    .map_err(IntegrateFsOpError::DeleteNode),
                Self::MoveNode(from_path, to_path) => fs
                    .move_node(&from_path, &to_path)
                    .await
                    .map_err(IntegrateFsOpError::MoveNode),
            }
        }
    }
}

trait FsExt: fs::Fs {
    fn contents_at_path(
        &self,
        path: &AbsPath,
    ) -> impl Future<
        Output = Result<Option<FsNodeContents>, ContentsAtPathError<Self>>,
    > + Send {
        async move {
            let node = match self.node_at_path(path).await {
                Ok(Some(node)) => node,
                Ok(None) => return Ok(None),
                Err(err) => return Err(ContentsAtPathError::NodeAtPath(err)),
            };

            Ok(Some(match &node {
                fs::Node::Directory(_) => FsNodeContents::Directory,

                fs::Node::File(file) => {
                    let contents = file
                        .read()
                        .await
                        .map_err(ContentsAtPathError::ReadFile)?;

                    match String::from_utf8(contents) {
                        Ok(contents) => FsNodeContents::Text(contents),
                        Err(err) => FsNodeContents::Binary(err.into_bytes()),
                    }
                },

                fs::Node::Symlink(symlink) => symlink
                    .read_path()
                    .await
                    .map(FsNodeContents::Symlink)
                    .map_err(ContentsAtPathError::ReadSymlink)?,
            }))
        }
    }
}

impl<Fs: fs::Fs> FsExt for Fs {}

fn text_diff(
    _lhs: crop::Rope,
    _rhs: &str,
) -> Option<SmallVec<[TextReplacement; 1]>> {
    todo!();
}

impl Messages {
    fn project_response(response: collab_types::ProjectResponse) -> Self {
        Self::ProjectResponse(Some(response))
    }

    fn renames(renames: SmallVec<[Rename; 2]>) -> Self {
        Self::Renames(renames.into_iter())
    }
}

impl Iterator for Messages {
    type Item = Message;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::None => None,
            Self::ProjectResponse(response) => {
                response.take().map(Message::ProjectResponse)
            },
            Self::Renames(iter) => iter.next().map(Message::RenamedFsNode),
        }
    }
}
