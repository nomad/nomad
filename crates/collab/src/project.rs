//! TODO: docs.

use core::marker::PhantomData;
use core::{fmt, iter};
use std::collections::hash_map;
use std::sync::Arc;

use abs_path::{AbsPath, AbsPathBuf};
use collab_project::fs::{
    DirectoryId,
    File,
    FileId,
    FileMut,
    FsOp,
    GlobalFileId,
    Node,
    NodeMut,
    NodeRename,
};
use collab_project::{PeerId, binary, text};
use collab_server::message::{self, Message, Peer};
use compact_str::format_compact;
use ed::fs::{self, File as _, Fs, FsNode, Symlink as _};
use ed::{AgentId, Buffer, Context, Editor, Shared, notify};
use fxhash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use smol_str::ToSmolStr;

use crate::CollabEditor;
use crate::convert::Convert;
use crate::editors::{ActionForSelectedSession, SessionId};
use crate::event::{
    BufferEvent,
    CursorEvent,
    CursorEventKind,
    Event,
    SelectionEvent,
    SelectionEventKind,
};

/// TODO: docs.
#[derive(cauchy::Clone)]
pub struct ProjectHandle<Ed: CollabEditor> {
    inner: Shared<Project<Ed>>,
    projects: Projects<Ed>,
}

/// TODO: docs.
#[derive(Debug, PartialEq)]
pub struct OverlappingProjectError {
    /// TODO: docs.
    pub existing_root: AbsPathBuf,

    /// TODO: docs.
    pub new_root: AbsPathBuf,
}

/// TODO: docs.
pub struct NoActiveSessionError<B>(PhantomData<B>);

/// TODO: docs.
pub(crate) struct Project<Ed: CollabEditor> {
    agent_id: AgentId,
    /// The [`PeerId`] of the host of the session.
    host_id: PeerId,
    /// Contains various mappings between editor IDs and project IDs.
    id_maps: IdMaps<Ed>,
    /// The inner CRDT holding the entire state of the project.
    inner: collab_project::Project,
    /// The ID of the local [`Peer`].
    local_peer_id: PeerId,
    /// Map from a remote selections's ID to the corresponding selection
    /// displayed in the editor.
    peer_selections: FxHashMap<text::SelectionId, Ed::PeerSelection>,
    /// Map from a remote cursor's ID to the corresponding tooltip displayed in
    /// the editor.
    peer_tooltips: FxHashMap<text::CursorId, Ed::PeerTooltip>,
    /// Map from a peer's ID to the corresponding [`Peer`]. Contains both the
    /// local peer and the remote peers.
    peers: FxHashMap<PeerId, Peer>,
    /// The path to the root of the project.
    root_path: AbsPathBuf,
    /// The ID of the collaborative session this project is part of.
    session_id: SessionId<Ed>,
}

#[derive(cauchy::Clone, cauchy::Default)]
pub(crate) struct Projects<Ed: CollabEditor> {
    active: Shared<FxHashMap<SessionId<Ed>, ProjectHandle<Ed>>>,
    starting: Shared<FxHashSet<AbsPathBuf>>,
}

pub(crate) struct ProjectGuard<Ed: CollabEditor> {
    root: AbsPathBuf,
    projects: Projects<Ed>,
}

pub(crate) struct NewProjectArgs<Ed: CollabEditor> {
    pub(crate) agent_id: AgentId,
    pub(crate) host_id: PeerId,
    pub(crate) id_maps: IdMaps<Ed>,
    pub(crate) local_peer: Peer,
    pub(crate) remote_peers: message::Peers,
    pub(crate) project: collab_project::Project,
    pub(crate) session_id: SessionId<Ed>,
}

#[derive(cauchy::Default)]
pub(crate) struct IdMaps<Ed: Editor> {
    pub(crate) buffer2file: FxHashMap<Ed::BufferId, FileId>,
    pub(crate) cursor2cursor: FxHashMap<Ed::CursorId, text::CursorId>,
    pub(crate) file2buffer: FxHashMap<FileId, Ed::BufferId>,
    pub(crate) node2dir: FxHashMap<<Ed::Fs as fs::Fs>::NodeId, DirectoryId>,
    pub(crate) node2file: FxHashMap<<Ed::Fs as fs::Fs>::NodeId, FileId>,
    pub(crate) selection2selection:
        FxHashMap<Ed::SelectionId, text::SelectionId>,
}

#[derive(cauchy::Debug)]
pub(crate) enum SynchronizeError<Ed: CollabEditor> {
    /// TODO: docs.
    ContentsAtPath(ContentsAtPathError<Ed::Fs>),
}

/// The type of error that can occcur when integrating a
/// [`binary::BinaryEdit`].
#[derive(cauchy::Debug)]
pub enum IntegrateBinaryEditError<Fs: fs::Fs> {
    /// The node at the given path was a directory, not a file.
    DirectoryAtPath(AbsPathBuf),
    /// It wasn't possible to get the node at the given path.
    NodeAtPath(Fs::NodeAtPathError),
    /// There wasn't any node at the given path.
    NoNodeAtPath(AbsPathBuf),
    /// The node at the given path was a symlink, not a file.
    SymlinkAtPath(AbsPathBuf),
    /// It wasn't possible to write the new contents to the file at the given
    /// path.
    WriteToFile(AbsPathBuf, <Fs::File as fs::File>::WriteError),
}

/// The type of error that can occcur when integrating a
/// [`binary::BinaryEdit`].
#[derive(cauchy::Debug)]
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

enum FsNodeContents {
    Directory,
    Text(String),
    Binary(Vec<u8>),
    Symlink(String),
}

impl<Ed: CollabEditor> ProjectHandle<Ed> {
    /// TODO: docs.
    pub fn is_host(&self) -> bool {
        self.with(|proj| proj.is_host())
    }

    /// TODO: docs.
    pub fn root(&self) -> AbsPathBuf {
        self.with(|proj| proj.root_path.clone())
    }

    /// TODO: docs.
    pub fn session_id(&self) -> SessionId<Ed> {
        self.with(|proj| proj.session_id)
    }

    pub(crate) fn handle_request(
        &self,
        request: message::ProjectRequest,
    ) -> message::ProjectResponse {
        let (peers, project) = self.with_project(|proj| {
            let peers = proj.peers.values().cloned().collect();
            let project = Box::new(proj.inner.clone().into_state());
            (peers, project)
        });

        message::ProjectResponse {
            peers,
            project,
            respond_to: request.requested_by.id,
        }
    }

    /// TODO: docs.
    #[allow(clippy::too_many_lines)]
    pub(crate) async fn integrate(
        &self,
        message: Message,
        ctx: &mut Context<Ed>,
    ) {
        match message {
            Message::CreatedCursor(cursor_creation) => {
                self.integrate_cursor_creation(cursor_creation, ctx).await
            },
            Message::CreatedDirectory(directory_creation) => {
                let _ = self.integrate_fs_op(directory_creation, ctx).await;
            },
            Message::CreatedFile(file_creation) => {
                let _ = self.integrate_fs_op(file_creation, ctx).await;
            },
            Message::CreatedSelection(selection_creation) => {
                self.integrate_selection_creation(selection_creation, ctx)
                    .await
            },
            Message::DeletedCursor(cursor_deletion) => {
                self.integrate_cursor_deletion(cursor_deletion, ctx).await
            },
            Message::DeletedFsNode(deletion) => {
                let _ = self.integrate_fs_op(deletion, ctx).await;
            },
            Message::DeletedSelection(selection_deletion) => {
                self.integrate_selection_deletion(selection_deletion, ctx)
                    .await;
            },
            Message::EditedBinary(binary_edit) => {
                let _ = self.integrate_binary_edit(binary_edit, ctx).await;
            },
            Message::EditedText(text_edit) => {
                self.integrate_text_edit(text_edit, ctx).await
            },
            Message::MovedCursor(cursor_movement) => {
                self.integrate_cursor_movement(cursor_movement, ctx).await
            },
            Message::MovedFsNode(movement) => {
                let _ = self.integrate_fs_op(movement, ctx).await;
            },
            Message::MovedSelection(selection_movement) => {
                self.integrate_selection_movement(selection_movement, ctx)
                    .await;
            },
            Message::PeerDisconnected(peer_id) => {
                self.integrate_peer_left(peer_id, ctx).await
            },
            Message::PeerJoined(peer) => {
                self.integrate_peer_joined(peer, ctx).await
            },
            Message::PeerLeft(peer_id) => {
                self.integrate_peer_left(peer_id, ctx).await
            },
            Message::ProjectRequest(_) => {
                panic!(
                    "ProjectRequest should've been handled by calling \
                     handle_request() instead of integrate()"
                );
            },
            Message::ProjectResponse(_) => {
                ctx.emit_error(notify::Message::from_display(
                    "received unexpected ProjectResponse message",
                ));
            },
            Message::SavedTextFile(file_id) => {
                let _ = self.integrate_file_save(file_id, ctx).await;
            },
        }
    }

    /// TODO: docs.
    pub(crate) async fn synchronize(
        &self,
        event: Event<Ed>,
        ctx: &mut Context<Ed>,
    ) -> Result<Option<Message>, SynchronizeError<Ed>> {
        match event {
            Event::Directory(fs::DirectoryEvent::Creation(creation)) => {
                self.synchronize_node_creation(creation, ctx).await
            },
            Event::File(fs::FileEvent::Modification(modification)) => {
                self.synchronize_file_modification(modification, ctx).await
            },
            other => Ok(self.with_project(|proj| proj.synchronize(other))),
        }
    }

    async fn integrate_binary_edit(
        &self,
        edit: binary::BinaryEdit,
        ctx: &mut Context<Ed>,
    ) -> Result<(), IntegrateBinaryEditError<Ed::Fs>> {
        let Some((file_path, new_contents)) = self.with_project(|proj| {
            let file_mut = proj.inner.integrate_binary_edit(edit)?;
            let file = file_mut.as_file();
            let file_path = proj.root_path.clone().concat(file.path());
            let new_contents = file.contents().to_owned();
            Some((file_path, new_contents))
        }) else {
            return Ok(());
        };

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
                FsNode::File(file) => file,
                FsNode::Directory(_) => {
                    return Err(IntegrateBinaryEditError::DirectoryAtPath(
                        file_path,
                    ));
                },
                FsNode::Symlink(_) => {
                    return Err(IntegrateBinaryEditError::SymlinkAtPath(
                        file_path,
                    ));
                },
            };

            file.write(new_contents).await.map_err(|err| {
                IntegrateBinaryEditError::WriteToFile(file_path, err)
            })
        })
        .await
    }

    async fn integrate_cursor_creation(
        &self,
        creation: text::CursorCreation,
        ctx: &mut Context<Ed>,
    ) {
        let Some((peer, offset, buf_id, cur_id)) = self.with_project(|proj| {
            let cursor = proj.inner.integrate_cursor_creation(creation)?;
            let cursor_file = cursor.file()?;
            let cursor_owner = proj.peers.get(&cursor.owner())?;
            let buf_id = proj.id_maps.file2buffer.get(&cursor_file.id())?;
            Some((
                cursor_owner.clone(),
                cursor.offset(),
                buf_id.clone(),
                cursor.id(),
            ))
        }) else {
            return;
        };

        let peer_tooltip =
            Ed::create_peer_tooltip(peer, offset, buf_id, ctx).await;

        self.with_project(|proj| {
            proj.peer_tooltips.insert(cur_id, peer_tooltip);
        });
    }

    async fn integrate_cursor_deletion(
        &self,
        deletion: text::CursorDeletion,
        ctx: &mut Context<Ed>,
    ) {
        let Some(tooltip) = self.with_project(|proj| {
            proj.inner
                .integrate_cursor_deletion(deletion)
                .and_then(|cursor_id| proj.peer_tooltips.remove(&cursor_id))
        }) else {
            return;
        };

        Ed::remove_peer_tooltip(tooltip, ctx).await;
    }

    async fn integrate_cursor_movement(
        &self,
        movement: text::CursorMovement,
        ctx: &mut Context<Ed>,
    ) {
        let Some(move_tooltip) = self.with_project(|proj| {
            let cursor = proj.inner.integrate_cursor_movement(movement)?;
            let tooltip = proj.peer_tooltips.get_mut(&cursor.id())?;
            Some(Ed::move_peer_tooltip(tooltip, cursor.offset(), ctx))
        }) else {
            return;
        };

        move_tooltip.await;
    }

    async fn integrate_file_save(
        &self,
        global_id: GlobalFileId,
        ctx: &mut Context<Ed>,
    ) -> Result<(), Ed::BufferSaveError> {
        let Some((buf_id, agent_id)) = self.with_project(|proj| {
            let file_id = proj.inner.local_file_of_global(global_id)?;
            let buf_id = proj.id_maps.file2buffer.get(&file_id)?.clone();
            Some((buf_id, proj.agent_id))
        }) else {
            return Ok(());
        };

        ctx.with_borrowed(|ctx| {
            let mut buffer = ctx.buffer(buf_id).expect("invalid buffer ID");
            if Ed::should_remote_save_cause_local_save(&buffer) {
                buffer.save(agent_id)
            } else {
                Ok(())
            }
        })
    }

    /// TODO: docs.
    async fn integrate_fs_op<T: FsOp>(
        &self,
        op: T,
        ctx: &mut Context<Ed>,
    ) -> Result<SmallVec<[NodeRename; 2]>, IntegrateFsOpError<Ed::Fs>> {
        use impl_integrate_fs_op as r#impl;

        let mut actions = SmallVec::new();
        let mut renames = SmallVec::new();

        self.with_project(|proj| {
            let mut sync_actions = proj.inner.integrate_fs_op(op);

            while let Some(sync_action) = sync_actions.next() {
                if let Some(more_renames) = r#impl::push_resolved_actions(
                    sync_action,
                    &proj.peers,
                    &mut actions,
                ) {
                    renames.extend(more_renames);
                }
            }
        });

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

    async fn integrate_peer_joined(&self, peer: Peer, _ctx: &mut Context<Ed>) {
        self.with_project(|proj| match proj.peers.entry(peer.id) {
            hash_map::Entry::Occupied(_) => {
                panic!("peer ID {:?} already exists", peer.id);
            },
            hash_map::Entry::Vacant(entry) => {
                entry.insert(peer);
            },
        });
    }

    async fn integrate_peer_left(
        &self,
        peer_id: PeerId,
        ctx: &mut Context<Ed>,
    ) {
        let (tooltips, _peer) = self.with_project(|proj| {
            let (cursor_ids, _selection_ids) =
                proj.inner.integrate_peer_disconnection(peer_id);

            let tooltips = cursor_ids
                .into_iter()
                .flat_map(|cursor_id| proj.peer_tooltips.remove(&cursor_id))
                .collect::<SmallVec<[_; 1]>>();

            let peer = match proj.peers.remove(&peer_id) {
                Some(peer) => peer,
                None => panic!("peer ID {peer_id:?} doesn't exist"),
            };

            (tooltips, peer)
        });

        for tooltip in tooltips {
            Ed::remove_peer_tooltip(tooltip, ctx).await;
        }
    }

    async fn integrate_selection_creation(
        &self,
        creation: text::SelectionCreation,
        ctx: &mut Context<Ed>,
    ) {
        let Some((peer, range, buf_id, sel_id)) = self.with_project(|proj| {
            let selection =
                proj.inner.integrate_selection_creation(creation)?;
            let file_id = selection.file()?.id();
            let buf_id = proj.id_maps.file2buffer.get(&file_id)?;
            let selection_owner = proj.peers.get(&selection.owner())?;
            Some((
                selection_owner.clone(),
                selection.offset_range(),
                buf_id.clone(),
                selection.id(),
            ))
        }) else {
            return;
        };

        let peer_selection =
            Ed::create_peer_selection(peer, range, buf_id, ctx).await;

        self.with_project(|proj| {
            proj.peer_selections.insert(sel_id, peer_selection);
        });
    }

    async fn integrate_selection_deletion(
        &self,
        deletion: text::SelectionDeletion,
        ctx: &mut Context<Ed>,
    ) {
        let Some(selection) = self.with_project(|proj| {
            proj.inner.integrate_selection_deletion(deletion).and_then(
                |selection_id| proj.peer_selections.remove(&selection_id),
            )
        }) else {
            return;
        };

        Ed::remove_peer_selection(selection, ctx).await;
    }

    async fn integrate_selection_movement(
        &self,
        movement: text::SelectionMovement,
        ctx: &mut Context<Ed>,
    ) {
        let Some(move_selection) = self.with_project(|proj| {
            let selection =
                proj.inner.integrate_selection_movement(movement)?;
            let peer_selection =
                proj.peer_selections.get_mut(&selection.id())?;
            Some(Ed::move_peer_selection(
                peer_selection,
                selection.offset_range(),
                ctx,
            ))
        }) else {
            return;
        };

        move_selection.await;
    }

    async fn integrate_text_edit(
        &self,
        edit: text::TextEdit,
        ctx: &mut Context<Ed>,
    ) {
        let Some((buf_id_or_file_path, replacements, agent_id)) = self
            .with_project(|proj| {
                let (file, replacements) =
                    proj.inner.integrate_text_edit(edit)?;
                let file_id = file.as_file().id();
                let buf_id_or_file_path = proj
                    .id_maps
                    .file2buffer
                    .get(&file_id)
                    .cloned()
                    .ok_or_else(|| {
                        let file = proj.inner.file(file_id).expect("is valid");
                        proj.root_path.clone().concat(file.path())
                    });
                Some((buf_id_or_file_path, replacements, proj.agent_id))
            })
        else {
            return;
        };

        // If there's already an open buffer for the edited file we can just
        // apply the replacements to it. If not, we have to first create one.
        let buffer_id = match buf_id_or_file_path {
            Ok(buf_id) => buf_id,
            // Not actually an error, we're abusing Result as an Either.
            Err(file_path) => {
                match ctx.create_buffer(&file_path, agent_id).await {
                    Ok(buf_id) => buf_id,
                    Err(err) => todo!("handle {err:?}"),
                }
            },
        };

        ctx.with_borrowed(|ctx| {
            ctx.buffer(buffer_id).expect("buffer exists").edit(
                replacements.into_iter().map(Convert::convert),
                agent_id,
            );
        });
    }

    #[allow(clippy::too_many_lines)]
    async fn synchronize_file_modification(
        &self,
        modification: fs::FileModification<Ed::Fs>,
        ctx: &mut Context<Ed>,
    ) -> Result<Option<Message>, SynchronizeError<Ed>> {
        enum FileContents {
            Binary(Arc<[u8]>),
            Text(text::crop::Rope),
        }

        enum FileDiff {
            Binary(Vec<u8>),
            Text(SmallVec<[text::TextReplacement; 1]>),
        }

        // Get the file's contents before the modification.
        let (file_id, file_path, file_contents) = self.with_project(|proj| {
            let root_path = proj.root_path.clone();

            match proj.project_node(&modification.file_id) {
                NodeMut::File(FileMut::Binary(file_mut)) => {
                    let file = file_mut.as_file();
                    let content = FileContents::Binary(file.contents().into());
                    (file.id(), root_path.concat(file.path()), content)
                },
                NodeMut::File(FileMut::Text(file_mut)) => {
                    let file = file_mut.as_file();
                    let contents = FileContents::Text(file.contents().clone());
                    (file.id(), root_path.concat(file.path()), contents)
                },
                NodeMut::File(FileMut::Symlink(_)) => {
                    panic!("received a FileModification event on a symlink")
                },
                NodeMut::Directory(_) => {
                    panic!("received a FileModification event on a directory")
                },
            }
        });

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
        Ok(self.with_project(|proj| {
            let file = proj.inner.file_mut(file_id)?;

            Some(match (file, file_diff) {
                (FileMut::Binary(mut file), FileDiff::Binary(contents)) => {
                    Message::EditedBinary(file.replace(contents))
                },
                (FileMut::Text(mut file), FileDiff::Text(replacements)) => {
                    Message::EditedText(file.edit(replacements))
                },
                _ => unreachable!(),
            })
        }))
    }

    async fn synchronize_node_creation(
        &self,
        creation: fs::NodeCreation<Ed::Fs>,
        ctx: &mut Context<Ed>,
    ) -> Result<Option<Message>, SynchronizeError<Ed>> {
        match ctx.fs().contents_at_path(&creation.node_path).await {
            Ok(Some(node_contents)) => Ok(Some(self.with_project(|proj| {
                proj.synchronize_node_creation(
                    creation.node_id,
                    &creation.node_path,
                    node_contents,
                )
            }))),

            // The node must've already been deleted or moved.
            //
            // FIXME: doing nothing can be problematic if we're about to
            // receive deletions/moves for the node.
            Ok(None) => Ok(None),

            Err(err) => Err(SynchronizeError::ContentsAtPath(err)),
        }
    }

    /// TODO: docs.
    fn with<R>(&self, fun: impl FnOnce(&Project<Ed>) -> R) -> R {
        self.inner.with(fun)
    }

    /// TODO: docs.
    fn with_project<R>(&self, fun: impl FnOnce(&mut Project<Ed>) -> R) -> R {
        self.inner.with_mut(fun)
    }
}

impl<Ed: CollabEditor> Project<Ed> {
    fn synchronize(&mut self, event: Event<Ed>) -> Option<Message> {
        match event {
            Event::Buffer(event) => self.synchronize_buffer(event),
            Event::Cursor(event) => Some(self.synchronize_cursor(event)),
            Event::Directory(event) => Some(self.synchronize_directory(event)),
            Event::File(event) => {
                self.synchronize_file(event);
                None
            },
            Event::Selection(event) => Some(self.synchronize_selection(event)),
        }
    }

    /// Returns the [`text::CursorMut`] corresponding to the cursor with the
    /// given ID.
    #[track_caller]
    fn cursor_of_cursor_id(
        &mut self,
        cursor_id: &Ed::CursorId,
    ) -> text::CursorMut<'_> {
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

    fn is_host(&self) -> bool {
        self.inner.peer_id() == self.host_id
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
    ) -> text::SelectionMut<'_> {
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

    fn synchronize_buffer(
        &mut self,
        event: BufferEvent<Ed>,
    ) -> Option<Message> {
        match event {
            BufferEvent::Created(buffer_id, file_path) => {
                let path_in_proj = file_path
                    .strip_prefix(&self.root_path)
                    .expect("the buffer is backed by a file in the project");

                let file_id = match self.inner.node_at_path(path_in_proj)? {
                    Node::File(file) => file.id(),
                    Node::Directory(_) => return None,
                };

                let ids = &mut self.id_maps;
                ids.buffer2file.insert(buffer_id.clone(), file_id);
                ids.file2buffer.insert(file_id, buffer_id);

                None
            },
            BufferEvent::Edited(buffer_id, replacements) => {
                let text_edit = self
                    .text_file_of_buffer(&buffer_id)
                    .edit(replacements.into_iter().map(Convert::convert));

                Some(Message::EditedText(text_edit))
            },
            BufferEvent::Removed(buffer_id) => {
                let ids = &mut self.id_maps;
                if let Some(file_id) = ids.buffer2file.remove(&buffer_id) {
                    ids.file2buffer.remove(&file_id);
                }
                None
            },
            BufferEvent::Saved(buffer_id) => {
                let file_id =
                    self.text_file_of_buffer(&buffer_id).as_file().global_id();

                Some(Message::SavedTextFile(file_id))
            },
        }
    }

    fn synchronize_cursor(&mut self, event: CursorEvent<Ed>) -> Message {
        match event.kind {
            CursorEventKind::Created(buffer_id, byte_offset) => {
                let (cursor_id, creation) = self
                    .text_file_of_buffer(&buffer_id)
                    .create_cursor(byte_offset);

                self.id_maps.cursor2cursor.insert(event.cursor_id, cursor_id);

                Message::CreatedCursor(creation)
            },
            CursorEventKind::Moved(byte_offset) => {
                let movement = self
                    .cursor_of_cursor_id(&event.cursor_id)
                    .r#move(byte_offset);

                Message::MovedCursor(movement)
            },
            CursorEventKind::Removed => {
                let deletion =
                    self.cursor_of_cursor_id(&event.cursor_id).delete();

                self.id_maps.cursor2cursor.remove(&event.cursor_id);

                Message::DeletedCursor(deletion)
            },
        }
    }

    fn synchronize_directory(
        &mut self,
        event: fs::DirectoryEvent<Ed::Fs>,
    ) -> Message {
        match event {
            fs::DirectoryEvent::Creation(_creation) => {
                unreachable!("already handled by ProjectHandle::synchronize()")
            },
            fs::DirectoryEvent::Deletion(deletion) => {
                self.synchronize_node_deletion(deletion)
            },
            fs::DirectoryEvent::Move(r#move) => {
                self.synchronize_node_move(r#move)
            },
        }
    }

    fn synchronize_file(&mut self, event: fs::FileEvent<Ed::Fs>) {
        match event {
            fs::FileEvent::Modification(_modification) => {
                unreachable!("already handled by ProjectHandle::synchronize()")
            },
            fs::FileEvent::IdChange(id_change) => {
                self.synchronize_file_id_change(id_change);
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

    fn synchronize_node_creation(
        &mut self,
        node_id: <Ed::Fs as fs::Fs>::NodeId,
        node_path: &AbsPath,
        node_contents: FsNodeContents,
    ) -> Message {
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

        let (file_mut, creation) = match node_contents {
            FsNodeContents::Directory => {
                match parent.create_directory(node_name) {
                    Ok((dir_mut, creation)) => {
                        let dir_id = dir_mut.as_directory().id();
                        self.id_maps.node2dir.insert(node_id, dir_id);
                        return Message::CreatedDirectory(creation);
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
        }
        .expect("no duplicate node names");

        let file_id = file_mut.as_file().id();
        self.id_maps.node2file.insert(node_id, file_id);
        Message::CreatedFile(creation)
    }

    fn synchronize_node_deletion(
        &mut self,
        deletion: fs::NodeDeletion<Ed::Fs>,
    ) -> Message {
        let node_id = deletion.node_id;
        let deletion = match self.project_node(&node_id) {
            NodeMut::Directory(dir) => {
                dir.delete().expect("dir is not the project root")
            },
            NodeMut::File(file) => file.delete(),
        };

        let ids = &mut self.id_maps;
        if let Some(file_id) = ids.node2file.remove(&node_id) {
            if let Some(buffer_id) = ids.file2buffer.remove(&file_id) {
                ids.buffer2file.remove(&buffer_id);
            }
        } else {
            ids.node2dir.remove(&node_id);
        }

        Message::DeletedFsNode(deletion)
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

        let movement = match self.project_node(&r#move.node_id) {
            NodeMut::Directory(mut dir) => {
                dir.r#move(parent_id).expect("invalid move on directory")
            },

            NodeMut::File(mut file) => {
                file.r#move(parent_id).expect("invalid move on file")
            },
        };

        Message::MovedFsNode(movement)
    }

    fn synchronize_selection(&mut self, event: SelectionEvent<Ed>) -> Message {
        match event.kind {
            SelectionEventKind::Created(buffer_id, byte_range) => {
                let (selection_id, creation) = self
                    .text_file_of_buffer(&buffer_id)
                    .create_selection(byte_range);

                self.id_maps
                    .selection2selection
                    .insert(event.selection_id, selection_id);

                Message::CreatedSelection(creation)
            },
            SelectionEventKind::Moved(byte_range) => {
                let movement = self
                    .selection_of_selection_id(&event.selection_id)
                    .r#move(byte_range);

                Message::MovedSelection(movement)
            },
            SelectionEventKind::Removed => {
                let deletion = self
                    .selection_of_selection_id(&event.selection_id)
                    .delete();

                self.id_maps.selection2selection.remove(&event.selection_id);

                Message::DeletedSelection(deletion)
            },
        }
    }

    /// Returns the [`text::TextFileMut`] corresponding to the buffer with the
    /// given ID.
    #[track_caller]
    fn text_file_of_buffer(
        &mut self,
        buffer_id: &Ed::BufferId,
    ) -> text::TextFileMut<'_> {
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
}

impl<Ed: CollabEditor> Projects<Ed> {
    pub(crate) fn get(
        &self,
        session_id: SessionId<Ed>,
    ) -> Option<ProjectHandle<Ed>> {
        self.active.with(|map| map.get(&session_id).cloned())
    }

    pub(crate) fn new_guard(
        &self,
        project_root: AbsPathBuf,
    ) -> Result<ProjectGuard<Ed>, OverlappingProjectError> {
        fn overlaps(l: &AbsPath, r: &AbsPath) -> bool {
            l.starts_with(r) || r.starts_with(l)
        }

        let conflicting_root = self
            .active
            .with(|map| {
                map.values().find_map(|handle| {
                    handle.with(|proj| {
                        overlaps(&proj.root_path, &project_root)
                            .then(|| proj.root_path.clone())
                    })
                })
            })
            .or_else(|| {
                self.starting.with(|roots| {
                    roots
                        .iter()
                        .find(|root| overlaps(root, &project_root))
                        .cloned()
                })
            });

        if let Some(conflicting_root) = conflicting_root {
            return Err(OverlappingProjectError {
                existing_root: conflicting_root,
                new_root: project_root,
            });
        }

        let guard = ProjectGuard {
            root: project_root.clone(),
            projects: self.clone(),
        };

        self.starting.with_mut(|map| {
            assert!(map.insert(project_root));
        });

        Ok(guard)
    }

    pub(crate) async fn select(
        &self,
        action: ActionForSelectedSession,
        ctx: &mut Context<Ed>,
    ) -> Result<Option<(AbsPathBuf, SessionId<Ed>)>, NoActiveSessionError<Ed>>
    {
        let active_sessions = self.active.with(|map| {
            map.iter()
                .map(|(session_id, handle)| {
                    let root = handle.with(|proj| proj.root_path.clone());
                    (root, *session_id)
                })
                .collect::<SmallVec<[_; 1]>>()
        });

        let session = match &*active_sessions {
            [] => return Err(NoActiveSessionError::new()),
            [single] => single,
            sessions => {
                match Ed::select_session(sessions, action, ctx).await {
                    Some(session) => session,
                    None => return Ok(None),
                }
            },
        };

        Ok(Some(session.clone()))
    }

    fn insert(&self, project: Project<Ed>) -> ProjectHandle<Ed> {
        let session_id = project.session_id;
        let handle = ProjectHandle {
            inner: Shared::new(project),
            projects: self.clone(),
        };
        self.active.with_mut(|map| {
            let prev = map.insert(session_id, handle.clone());
            assert!(prev.is_none());
        });
        handle
    }
}

impl<Ed: CollabEditor> ProjectGuard<Ed> {
    pub(crate) fn activate(
        self,
        args: NewProjectArgs<Ed>,
    ) -> ProjectHandle<Ed> {
        self.projects.starting.with_mut(|set| {
            assert!(set.remove(&self.root));
        });

        let local_peer_id = args.local_peer.id;

        let peers = args
            .remote_peers
            .into_iter()
            .map(|peer| (peer.id, peer))
            .chain(iter::once((local_peer_id, args.local_peer)))
            .collect();

        self.projects.insert(Project {
            agent_id: args.agent_id,
            host_id: args.host_id,
            id_maps: args.id_maps,
            inner: args.project,
            local_peer_id,
            peer_selections: FxHashMap::default(),
            peer_tooltips: FxHashMap::default(),
            peers,
            root_path: self.root.clone(),
            session_id: args.session_id,
        })
    }

    pub(crate) fn root(&self) -> &AbsPath {
        &self.root
    }
}

mod impl_integrate_fs_op {
    //! Contains the various types, free-standing functions and methods used in
    //! the implementation of [`ProjectHandle::integrate_fs_op`].

    use abs_path::{NodeName, NodeNameBuf};
    use collab_project::fs::{AttachedOrSync, ResolveConflict, SyncAction};
    use ed::fs::Directory;

    use super::*;

    pub(super) enum FileContents {
        Binary(Arc<[u8]>),
        Text(text::crop::Rope),
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

    pub(super) fn push_resolved_actions(
        action: SyncAction<'_>,
        peers: &FxHashMap<PeerId, Peer>,
        actions: &mut SmallVec<[ResolvedFsAction; 1]>,
    ) -> Option<[NodeRename; 2]> {
        match action {
            SyncAction::Create(create) => {
                push_node_creation(create.node(), &mut actions);
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
            SyncAction::CreateAndResolve(create_and_resolve) => {
                let create_node = create_and_resolve.create().node();
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
                push_node_creation(create_node, &mut actions);

                let (rename_conflicting, rename_existing) =
                    resolve_naming_conflict(
                        create_and_resolve.into_resolve(),
                        NamingConflictSource::Creation,
                        peers,
                    );

                match &mut actions[orig_len] {
                    ResolvedFsAction::MoveNode(_, dest_path) => {
                        dest_path.pop();
                        dest_path.push(rename_existing.name());
                    },
                    _ => unreachable!("we pushed a MoveNode action above"),
                }

                match &mut actions[orig_len + 1] {
                    ResolvedFsAction::CreateDirectory(path)
                    | ResolvedFsAction::CreateFile(path, _) => {
                        path.pop();
                        path.push(rename_conflicting.name());
                    },
                    _ => unreachable!("we pushed a Create* action above"),
                }

                Some([rename_conflicting, rename_existing])
            },
            SyncAction::MoveAndResolve(move_and_resolve) => {
                let r#move = move_and_resolve.r#move();
                Some(push_move_and_resolve(
                    r#move.new_path(),
                    r#move.old_path(),
                    move_and_resolve.into_resolve(),
                    peers,
                    actions,
                ))
            },
            SyncAction::RenameAndResolve(rename_and_resolve) => {
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
        node: Node<impl AttachedOrSync>,
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
    fn push_move_and_resolve(
        move_existing_from: AbsPathBuf,
        move_conflicting_from: AbsPathBuf,
        conflict: ResolveConflict<'_>,
        peers: &FxHashMap<PeerId, Peer>,
        actions: &mut SmallVec<[ResolvedFsAction; 1]>,
    ) -> [NodeRename; 2] {
        let (rename_conflicting, rename_existing) = resolve_naming_conflict(
            conflict,
            NamingConflictSource::Movement,
            peers,
        );

        let move_existing_to = {
            let mut path = move_existing_from.clone();
            path.pop();
            path.push(rename_existing.name());
            path
        };

        let move_conflicting_to = {
            let mut path = move_existing_from.clone();
            path.pop();
            path.push(rename_conflicting.name());
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

    fn resolve_naming_conflict(
        mut conflict: ResolveConflict<'_>,
        conflict_source: NamingConflictSource,
        peers: &FxHashMap<PeerId, Peer>,
    ) -> (NodeRename, NodeRename) {
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
                peers.get(conflicting.created_by()),
                peers.get(existing.created_by()),
            ) {
                let gen_name =
                    |current_name: &NodeName, node_creator: &Peer| {
                        let suffix = node_creator.github_handle.as_str();
                        format_compact!("{current_name}-{suffix}")
                            .parse::<NodeNameBuf>()
                            .expect("new name is valid")
                    };

                let mut conflicting = conflict.conflicting_node_mut();
                let new_name =
                    gen_name(conflicting.name(), creator_conflicting);
                let rename_conflicting = conflicting.force_rename(new_name);

                let mut existing = conflict.existing_node_mut();
                let new_name = gen_name(existing.name(), creator_existing);
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
                (conflicting.created_by(), existing.created_by())
            } else {
                let seed = existing.created_by();
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

        let orig_name_conflicting = conflicting.name().to_owned();
        let orig_name_existing = existing.name().to_owned();

        loop {
            let mut conflicting = conflict.conflicting_node_mut();
            let new_name =
                gen_name(orig_name_conflicting, &mut rng_conflicting);
            let rename_conflicting = conflicting.force_rename(new_name);

            let mut existing = conflict.existing_node_mut();
            let new_name = gen_name(orig_name_existing, &mut rng_existing);
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

impl<B> NoActiveSessionError<B> {
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

impl<Ed: CollabEditor> Drop for ProjectHandle<Ed> {
    fn drop(&mut self) {
        // The Projects struct is also storing an instance of this
        // ProjectHandle, so if the strong count is 2 it effectively means
        // we're dropping the last used instance.
        if self.inner.strong_count() == 2 {
            // Removing the ProjectHandle from the Projects will cause this
            // Drop impl to be called again, so use a non-panicking method
            // to access the inner map.
            let _ = self.projects.active.try_with_mut(|map| {
                map.remove(&self.session_id());
            });
        }
    }
}

impl<Ed: CollabEditor> Drop for ProjectGuard<Ed> {
    fn drop(&mut self) {
        self.projects.starting.with_mut(|set| {
            set.remove(&self.root);
        });
    }
}

impl notify::Error for OverlappingProjectError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        let mut msg = notify::Message::new();
        msg.push_str("cannot start a new session at ")
            .push_info(self.new_root.to_smolstr())
            .push_str(", another one is already running at ")
            .push_info(self.existing_root.to_smolstr())
            .push_str(" (sessions cannot overlap)");
        (notify::Level::Error, msg)
    }
}

impl<B> fmt::Debug for NoActiveSessionError<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("NoActiveSessionError")
    }
}

impl<B> notify::Error for NoActiveSessionError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        let msg = "there's no active collaborative editing session";
        (notify::Level::Error, notify::Message::from_str(msg))
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
                fs::FsNode::Directory(_) => FsNodeContents::Directory,

                fs::FsNode::File(file) => {
                    let contents = file
                        .read()
                        .await
                        .map_err(ContentsAtPathError::ReadFile)?;

                    match String::from_utf8(contents) {
                        Ok(contents) => FsNodeContents::Text(contents),
                        Err(err) => FsNodeContents::Binary(err.into_bytes()),
                    }
                },

                fs::FsNode::Symlink(symlink) => symlink
                    .read_path()
                    .await
                    .map(FsNodeContents::Symlink)
                    .map_err(ContentsAtPathError::ReadSymlink)?,
            }))
        }
    }
}

impl<Fs: fs::Fs> FsExt for Fs {}

pub(crate) enum ContentsAtPathError<Fs: fs::Fs> {
    NodeAtPath(Fs::NodeAtPathError),
    ReadFile(<Fs::File as fs::File>::ReadError),
    ReadSymlink(<Fs::Symlink as fs::Symlink>::ReadError),
}

fn text_diff(
    _lhs: text::crop::Rope,
    _rhs: &str,
) -> Option<SmallVec<[text::TextReplacement; 1]>> {
    todo!();
}
