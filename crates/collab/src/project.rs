//! TODO: docs.

use core::fmt;
use core::marker::PhantomData;
use std::sync::Arc;

use abs_path::{AbsPath, AbsPathBuf};
use collab_project::fs::{DirectoryId, FileId, FileMut, Node, NodeMut};
use collab_project::{PeerId, text};
use collab_server::message::{GitHubHandle, Message, Peer, Peers};
use ed::backend::{AgentId, Backend};
use ed::fs::{self, File as _, Symlink as _};
use ed::{Context, Shared, notify};
use fxhash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use smol_str::ToSmolStr;

use crate::CollabBackend;
use crate::backend::{ActionForSelectedSession, SessionId};
use crate::convert::Convert;
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
pub struct ProjectHandle<B: CollabBackend> {
    inner: Shared<Project<B>>,
    is_dropping_last_instance: Shared<bool>,
    projects: Projects<B>,
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
pub(crate) struct Project<B: CollabBackend> {
    agent_id: AgentId,
    host_id: PeerId,
    id_maps: IdMaps<B>,
    local_peer: Peer,
    project: collab_project::Project,
    _remote_peers: Peers,
    root_path: AbsPathBuf,
    session_id: SessionId<B>,
}

#[derive(cauchy::Clone, cauchy::Default)]
pub(crate) struct Projects<B: CollabBackend> {
    active: Shared<FxHashMap<SessionId<B>, ProjectHandle<B>>>,
    starting: Shared<FxHashSet<AbsPathBuf>>,
}

pub(crate) struct ProjectGuard<B: CollabBackend> {
    root: AbsPathBuf,
    projects: Projects<B>,
}

pub(crate) struct NewProjectArgs<B: CollabBackend> {
    pub(crate) agent_id: AgentId,
    pub(crate) host_id: PeerId,
    pub(crate) id_maps: IdMaps<B>,
    pub(crate) local_peer: Peer,
    pub(crate) remote_peers: Peers,
    pub(crate) project: collab_project::Project,
    pub(crate) session_id: SessionId<B>,
}

#[derive(cauchy::Default)]
pub(crate) struct IdMaps<B: Backend> {
    pub(crate) buffer2file: FxHashMap<B::BufferId, FileId>,
    pub(crate) cursor2cursor: FxHashMap<CursorId<B>, text::CursorId>,
    pub(crate) file2buffer: FxHashMap<FileId, B::BufferId>,
    pub(crate) node2dir: FxHashMap<<B::Fs as fs::Fs>::NodeId, DirectoryId>,
    pub(crate) node2file: FxHashMap<<B::Fs as fs::Fs>::NodeId, FileId>,
    pub(crate) selection2selection:
        FxHashMap<SelectionId<B>, text::SelectionId>,
}

#[derive(cauchy::Debug, cauchy::PartialEq, cauchy::Eq, cauchy::Hash)]
pub(crate) struct CursorId<B: Backend> {
    buffer_id: B::BufferId,
    cursor_id: B::CursorId,
}

#[derive(cauchy::Debug, cauchy::PartialEq, cauchy::Eq, cauchy::Hash)]
pub(crate) struct SelectionId<B: Backend> {
    buffer_id: B::BufferId,
    selection_id: B::SelectionId,
}

#[derive(cauchy::Debug)]
pub(crate) enum SynchronizeError<B: CollabBackend> {
    /// TODO: docs.
    ContentsAtPath(ContentsAtPathError<B::Fs>),
}

enum FsNodeContents {
    Directory,
    Text(String),
    Binary(Vec<u8>),
    Symlink(String),
}

impl<B: CollabBackend> ProjectHandle<B> {
    /// TODO: docs.
    pub fn is_host(&self) -> bool {
        self.with(|proj| proj.is_host())
    }

    /// TODO: docs.
    pub fn root(&self) -> AbsPathBuf {
        self.with(|proj| proj.root_path.clone())
    }

    /// TODO: docs.
    pub fn session_id(&self) -> SessionId<B> {
        self.with(|proj| proj.session_id)
    }

    /// TODO: docs.
    pub(crate) async fn integrate(
        &self,
        _message: Message,
        _ctx: &mut Context<B>,
    ) {
        todo!();
    }

    /// TODO: docs.
    pub(crate) async fn synchronize(
        &self,
        event: Event<B>,
        ctx: &mut Context<B>,
    ) -> Result<Option<Message>, SynchronizeError<B>> {
        match event {
            Event::Directory(fs::DirectoryEvent::Creation(creation)) => {
                self.synchronize_node_creation(creation, ctx).await
            },
            Event::File(fs::FileEvent::Modification(modification)) => {
                self.synchronize_file_modification(modification, ctx).await
            },
            other => Ok(self.with_mut(|proj| proj.synchronize(other))),
        }
    }

    #[allow(clippy::too_many_lines)]
    async fn synchronize_file_modification(
        &self,
        modification: fs::FileModification<B::Fs>,
        ctx: &mut Context<B>,
    ) -> Result<Option<Message>, SynchronizeError<B>> {
        enum FileContents {
            Binary(Arc<[u8]>),
            Text(text::crop::Rope),
        }

        enum FileDiff {
            Binary(Vec<u8>),
            Text(SmallVec<[text::TextReplacement; 1]>),
        }

        // Get the file's contents before the modification.
        let (file_id, file_path, file_contents) = self.with_mut(|proj| {
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
        Ok(self.with_mut(|proj| {
            let file = proj.project.file_mut(file_id)?;

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
        creation: fs::NodeCreation<B::Fs>,
        ctx: &mut Context<B>,
    ) -> Result<Option<Message>, SynchronizeError<B>> {
        match ctx.fs().contents_at_path(&creation.node_path).await {
            Ok(Some(node_contents)) => Ok(Some(self.with_mut(|proj| {
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
    fn with<R>(&self, fun: impl FnOnce(&Project<B>) -> R) -> R {
        self.inner.with(fun)
    }

    /// TODO: docs.
    fn with_mut<R>(&self, fun: impl FnOnce(&mut Project<B>) -> R) -> R {
        self.inner.with_mut(fun)
    }
}

impl<B: CollabBackend> Project<B> {
    fn synchronize(&mut self, event: Event<B>) -> Option<Message> {
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
        cursor_id: &CursorId<B>,
    ) -> text::CursorMut<'_> {
        let Some(&project_cursor_id) =
            self.id_maps.cursor2cursor.get(cursor_id)
        else {
            panic!("unknown cursor ID: {cursor_id:?}");
        };

        let Ok(maybe_cursor) = self.project.cursor_mut(project_cursor_id)
        else {
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
        self.project.peer_id() == self.host_id
    }

    /// Returns the [`NodeMut`] corresponding to the node with the given
    /// ID.
    #[track_caller]
    fn project_node(
        &mut self,
        node_id: &<B::Fs as fs::Fs>::NodeId,
    ) -> NodeMut<'_> {
        if let Some(&dir_id) = self.id_maps.node2dir.get(node_id) {
            let Some(dir) = self.project.directory_mut(dir_id) else {
                panic!("node ID {node_id:?} maps to a deleted directory")
            };
            NodeMut::Directory(dir)
        } else if let Some(&file_id) = self.id_maps.node2file.get(node_id) {
            let Some(file) = self.project.file_mut(file_id) else {
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
        selection_id: &SelectionId<B>,
    ) -> text::SelectionMut<'_> {
        let Some(&project_selection_id) =
            self.id_maps.selection2selection.get(selection_id)
        else {
            panic!("unknown selection ID: {selection_id:?}");
        };

        let Ok(maybe_selection) =
            self.project.selection_mut(project_selection_id)
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
        event: BufferEvent<B>,
    ) -> Option<Message> {
        match event {
            BufferEvent::Created(buffer_id, file_path) => {
                let path_in_proj = file_path
                    .strip_prefix(&self.root_path)
                    .expect("the buffer is backed by a file in the project");

                let file_id = match self.project.node_at_path(path_in_proj)? {
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

    fn synchronize_cursor(&mut self, event: CursorEvent<B>) -> Message {
        match event.kind {
            CursorEventKind::Created(byte_offset) => {
                let (cursor_id, creation) = self
                    .text_file_of_buffer(&event.buffer_id)
                    .create_cursor(byte_offset.into());

                self.id_maps.cursor2cursor.insert(
                    CursorId {
                        buffer_id: event.buffer_id,
                        cursor_id: event.cursor_id,
                    },
                    cursor_id,
                );

                Message::CreatedCursor(creation)
            },
            CursorEventKind::Moved(byte_offset) => {
                let movement = self
                    .cursor_of_cursor_id(&CursorId {
                        buffer_id: event.buffer_id,
                        cursor_id: event.cursor_id,
                    })
                    .r#move(byte_offset.into());

                Message::MovedCursor(movement)
            },
            CursorEventKind::Removed => {
                let cursor_id = CursorId {
                    buffer_id: event.buffer_id,
                    cursor_id: event.cursor_id,
                };

                let deletion = self.cursor_of_cursor_id(&cursor_id).delete();

                self.id_maps.cursor2cursor.remove(&cursor_id);

                Message::DeletedCursor(deletion)
            },
        }
    }

    fn synchronize_directory(
        &mut self,
        event: fs::DirectoryEvent<B::Fs>,
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

    fn synchronize_file(&mut self, event: fs::FileEvent<B::Fs>) {
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
        id_change: fs::FileIdChange<B::Fs>,
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
        node_id: <B::Fs as fs::Fs>::NodeId,
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

        let Some(parent) =
            self.project.node_at_path_mut(parent_path_in_project)
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
        deletion: fs::NodeDeletion<B::Fs>,
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
        r#move: fs::NodeMove<B::Fs>,
    ) -> Message {
        let parent_path =
            r#move.new_path.parent().expect("root can't be moved");

        let parent_path_in_project = parent_path
            .strip_prefix(&self.root_path)
            .expect("the new parent has to be in the project");

        let Some(parent) =
            self.project.node_at_path_mut(parent_path_in_project)
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

    fn synchronize_selection(&mut self, event: SelectionEvent<B>) -> Message {
        match event.kind {
            SelectionEventKind::Created(byte_range) => {
                let (selection_id, creation) = self
                    .text_file_of_buffer(&event.buffer_id)
                    .create_selection(byte_range.convert());

                self.id_maps.selection2selection.insert(
                    SelectionId {
                        buffer_id: event.buffer_id,
                        selection_id: event.selection_id,
                    },
                    selection_id,
                );

                Message::CreatedSelection(creation)
            },
            SelectionEventKind::Moved(byte_range) => {
                let movement = self
                    .selection_of_selection_id(&SelectionId {
                        buffer_id: event.buffer_id,
                        selection_id: event.selection_id,
                    })
                    .r#move(byte_range.convert());

                Message::MovedSelection(movement)
            },
            SelectionEventKind::Removed => {
                let selection_id = SelectionId {
                    buffer_id: event.buffer_id,
                    selection_id: event.selection_id,
                };

                let deletion =
                    self.selection_of_selection_id(&selection_id).delete();

                self.id_maps.selection2selection.remove(&selection_id);

                Message::DeletedSelection(deletion)
            },
        }
    }

    /// Returns the [`text::TextFileMut`] corresponding to the buffer with the
    /// given ID.
    #[track_caller]
    fn text_file_of_buffer(
        &mut self,
        buffer_id: &B::BufferId,
    ) -> text::TextFileMut<'_> {
        let Some(&file_id) = self.id_maps.buffer2file.get(buffer_id) else {
            panic!("unknown buffer ID: {buffer_id:?}");
        };

        let Some(file) = self.project.file_mut(file_id) else {
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

impl<B: CollabBackend> Projects<B> {
    pub(crate) fn get(
        &self,
        session_id: SessionId<B>,
    ) -> Option<ProjectHandle<B>> {
        self.active.with(|map| map.get(&session_id).cloned())
    }

    pub(crate) fn new_guard(
        &self,
        project_root: AbsPathBuf,
    ) -> Result<ProjectGuard<B>, OverlappingProjectError> {
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
        ctx: &mut Context<B>,
    ) -> Result<Option<(AbsPathBuf, SessionId<B>)>, NoActiveSessionError<B>>
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
            sessions => match B::select_session(sessions, action, ctx).await {
                Some(session) => session,
                None => return Ok(None),
            },
        };

        Ok(Some(session.clone()))
    }

    fn insert(&self, project: Project<B>) -> ProjectHandle<B> {
        let session_id = project.session_id;
        let handle = ProjectHandle {
            inner: Shared::new(project),
            is_dropping_last_instance: Shared::new(false),
            projects: self.clone(),
        };
        self.active.with_mut(|map| {
            let prev = map.insert(session_id, handle.clone());
            assert!(prev.is_none());
        });
        handle
    }
}

impl<B: CollabBackend> ProjectGuard<B> {
    pub(crate) fn activate(self, args: NewProjectArgs<B>) -> ProjectHandle<B> {
        self.projects.starting.with_mut(|set| {
            assert!(set.remove(&self.root));
        });

        self.projects.insert(Project {
            agent_id: args.agent_id,
            host_id: args.host_id,
            id_maps: args.id_maps,
            local_peer: args.local_peer,
            _remote_peers: args.remote_peers,
            project: args.project,
            root_path: self.root.clone(),
            session_id: args.session_id,
        })
    }

    pub(crate) fn root(&self) -> &AbsPath {
        &self.root
    }
}

impl<B> NoActiveSessionError<B> {
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

impl<B: CollabBackend> Drop for ProjectHandle<B> {
    fn drop(&mut self) {
        if self.inner.strong_count() == 2
            && !self.is_dropping_last_instance.copied()
        {
            self.is_dropping_last_instance.set(true);

            self.projects.active.with_mut(|map| {
                map.remove(&self.session_id());
            });
        }
    }
}

impl<B: CollabBackend> Drop for ProjectGuard<B> {
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
