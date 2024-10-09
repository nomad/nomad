use core::fmt;
use std::collections::HashMap;
use std::io;

use collab_fs::{AbsUtf8Path, AbsUtf8PathBuf, Fs};
use collab_messaging::{Outbound, PeerId, Recipients};
use collab_project::file::{AnchorBias, FileId};
use collab_project::{actions, cursor, selection, Project};
use collab_server::JoinRequest;
use futures_util::stream::select_all;
use futures_util::{select, FutureExt, SinkExt, StreamExt};
use nohash::{IntMap as NoHashMap, IntSet as NoHashSet};
use nomad::{ByteOffset, Context, JoinHandle, Spawner, Subscription};
use nomad_server::client::{Joined, Receiver, Sender};
use nomad_server::{Io, Message, ProjectMessage};
use root_finder::markers::Git;
use root_finder::Finder;
use tracing::error;

use crate::events::cursor::{Cursor, CursorAction};
use crate::events::edit::Edit;
use crate::events::selection::{Selection, SelectionAction};
use crate::{CollabEditor, Config, SessionId};

pub(crate) struct Session<E: CollabEditor> {
    /// TODO: docs.
    config: Config,

    /// TODO: docs.
    editor: E,

    /// The session's ID.
    id: SessionId,

    /// The peers currently in the session, including the local peer but
    /// excluding the server.
    peers: NoHashSet<PeerId>,

    /// TODO: docs.
    project: Project,

    /// The path to the root of the project.
    project_root: AbsUtf8PathBuf,

    /// A receiver for receiving messages from the server.
    receiver: Receiver,

    /// The server's ID.
    server_id: PeerId,

    /// A sender for sending messages to the server.
    sender: Sender,

    /// TODO: docs.
    subs_edits: HashMap<E::FileId, E::Edits>,

    /// TODO: docs.
    subs_cursors: HashMap<E::FileId, E::Cursors>,

    /// TODO: docs.
    subs_selections: HashMap<E::FileId, E::Selections>,

    /// TODO: docs.
    cursors: Cursors<E>,

    /// TODO: docs.
    cursor_tooltips: HashMap<cursor::CursorId, CursorTooltip<E>>,

    /// TODO: docs.
    selections: Selections<E>,

    /// TODO: docs.
    selection_highlights:
        HashMap<selection::SelectionId, SelectionHighlight<E>>,
}

struct Cursors<E: CollabEditor> {
    local: HashMap<E::CursorId, cursor::Cursor>,
    remote: HashMap<cursor::CursorId, cursor::Cursor>,
}

struct Selections<E: CollabEditor> {
    local: HashMap<E::SelectionId, selection::Selection>,
    remote: HashMap<selection::SelectionId, selection::Selection>,
}

impl<E: CollabEditor> Session<E> {
    pub(crate) async fn join(
        id: SessionId,
        config: Config,
        ctx: Context<E>,
    ) -> Result<Self, JoinSessionError> {
        todo!();
        // let mut joined = Io::connect()
        //     .await?
        //     .authenticate(())
        //     .await?
        //     .join(JoinRequest::JoinExistingSession(id))
        //     .await?;
        //
        // let project = ask_for_project(&mut joined).await?;
        //
        // let project_root = config.nomad_dir().join(project.name());
        //
        // create_project_dir(&project, project_root, ctx.fs()).await?;
        //
        // // TODO: navigate to the project.
        // //
        // // focus_project_file(ctx, &project_root).await?;
        //
        // Ok(Self::new(config, ctx, joined, project, project_root))
    }

    pub(crate) async fn start(
        config: Config,
        ctx: Context<E>,
    ) -> Result<Self, StartSessionError> {
        todo!();
        // let Some(file) = ctx.buffer().file() else {
        //     return Err(StartSessionError::NotInFile);
        // };
        //
        // let Some(root_candidate) =
        //     Finder::find_root(file.path(), &Git, ctx.fs()).await?
        // else {
        //     return Err(StartSessionError::CouldntFindRoot(
        //         file.path().to_owned(),
        //     ));
        // };
        //
        // let project_root =
        //     match ctx.ask_user(ConfirmStart(&root_candidate)).await {
        //         Ok(true) => root_candidate,
        //         Ok(false) => return Err(StartSessionError::UserCancelled),
        //         Err(err) => return Err(err.into()),
        //     };
        //
        // let joined = Io::connect()
        //     .await?
        //     .authenticate(())
        //     .await?
        //     .join(JoinRequest::StartNewSession)
        //     .await?;
        //
        // let peer_id = joined.join_response.client_id;
        //
        // let project = Project::from_fs(peer_id, ctx.fs()).await?;
        //
        // Ok(Self::new(config, ctx, joined, project, project_root))
    }

    fn is_host(&self) -> bool {
        todo!()
    }

    fn peer_id(&self) -> PeerId {
        self.sender.peer_id()
    }

    fn new(
        config: Config,
        ctx: Context<E>,
        joined: Joined,
        project: Project,
        project_root: AbsUtf8PathBuf,
    ) -> Self {
        let Joined { sender, receiver, join_response, peers } = joined;
        todo!();
        // Self {
        //     config,
        //     cursors: Default::default(),
        //     // ctx,
        //     id: SessionId(join_response.session_id),
        //     peers,
        //     project,
        //     project_root,
        //     receiver,
        //     sender,
        //     server_id: join_response.server_id,
        //     subs_cursors: HashMap::new(),
        //     subs_edits: HashMap::new(),
        //     subs_selections: HashMap::new(),
        // }
    }
}

impl<E: CollabEditor> Session<E> {
    pub(crate) async fn run(mut self) -> Result<(), RunSessionError> {
        loop {
            let mut cursors = select_all(self.subs_cursors.values_mut());
            let mut edits = select_all(self.subs_edits.values_mut());
            let mut selections = select_all(self.subs_selections.values_mut());

            select! {
                cursor = cursors.next().fuse() => {
                    let cursor = cursor.expect("never ends");
                    drop(cursors);
                    drop(edits);
                    drop(selections);
                    self.sync_cursor(cursor).await?;
                },

                edit = edits.next().fuse() => {
                    let edit = edit.expect("never ends");
                    drop(cursors);
                    drop(edits);
                    drop(selections);
                    self.sync_edit(edit).await?;
                },

                selection = selections.next().fuse() => {
                    let selection = selection.expect("never ends");
                    drop(cursors);
                    drop(edits);
                    drop(selections);
                    self.sync_selection(selection).await?;
                },

                maybe_msg = self.receiver.next().fuse() => {
                    drop(cursors);
                    drop(edits);
                    drop(selections);
                    match maybe_msg {
                        Some(Ok(msg)) => self.integrate_message(msg).await?,
                        Some(Err(err)) => return Err(err.into()),
                        None => todo!(),
                    };
                },
            }
        }
    }

    async fn broadcast(
        &mut self,
        message: impl Into<Message>,
    ) -> Result<(), RunSessionError> {
        let outbound = Outbound {
            message: message.into(),
            recipients: Recipients::except([self.server_id]),
            should_compress: false,
        };

        self.sender.send(outbound).await.map_err(Into::into)
    }

    fn create_cursor_tooltip(
        &mut self,
        cursor: &cursor::Cursor,
    ) -> Option<CursorTooltip<E>> {
        let file_id = self.to_editor_file_id(cursor.file_id())?;
        let file = self.project.file(cursor.file_id())?;
        let offset = file.resolve_anchor(cursor.anchor())?;
        let owner_id = PeerId::new(cursor.owner().as_u64());
        let owner = self.peers.get(&owner_id)?;
        Some(CursorTooltip {
            cursor_id: cursor.id(),
            inner: self.editor.create_tooltip(
                &file_id,
                offset.into(),
                owner.into_u64(),
            ),
        })
    }

    fn create_selection_highlight(
        &mut self,
        selection: &selection::Selection,
    ) -> Option<SelectionHighlight<E>> {
        let file_id = self.to_editor_file_id(selection.file_id())?;
        let file = self.project.file(selection.file_id())?;
        let head = file.resolve_anchor(selection.head())?.into();
        let tail = file.resolve_anchor(selection.tail())?.into();
        let owner_id = PeerId::new(selection.owner().as_u64());
        let owner = self.peers.get(&owner_id)?;
        let byte_range = if head < tail { head..tail } else { tail..head };
        Some(SelectionHighlight {
            selection_id: selection.id(),
            inner: self.editor.create_highlight(
                &file_id,
                byte_range,
                (owner.into_u64() as u8, 0, 0),
            ),
        })
    }

    fn integrate_created_cursor(
        &mut self,
        msg: actions::create_cursor::CreatedCursorRemote,
    ) {
        let Some(cursor) = self.project.integrate(msg) else { return };
        if let Some(tooltip) = self.create_cursor_tooltip(&cursor) {
            self.cursor_tooltips.insert(cursor.id(), tooltip);
        }
        self.cursors.insert_remote(cursor);
    }

    async fn integrate_created_directory(
        &mut self,
        _msg: actions::create_directory::CreatedDirectoryRemote,
    ) -> Result<(), RunSessionError> {
        Ok(())
    }

    async fn integrate_created_file(
        &mut self,
        _msg: actions::create_file::CreatedFileRemote,
    ) -> Result<(), RunSessionError> {
        Ok(())
    }

    fn integrate_created_selection(
        &mut self,
        msg: actions::create_selection::CreatedSelectionRemote,
    ) {
        let Some(selection) = self.project.integrate(msg) else { return };
        if let Some(highlight) = self.create_selection_highlight(&selection) {
            self.selection_highlights.insert(selection.id(), highlight);
        }
        self.selections.insert_remote(selection);
    }

    async fn integrate_deleted_text(
        &mut self,
        _msg: actions::delete_text::DeletedTextRemote,
    ) -> Result<(), RunSessionError> {
        Ok(())
    }

    async fn integrate_inserted_text(
        &mut self,
        _msg: actions::insert_text::InsertedTextRemote,
    ) -> Result<(), RunSessionError> {
        Ok(())
    }

    fn integrate_moved_cursor(
        &mut self,
        msg: actions::move_cursor::MovedCursorRemote,
    ) {
        let Some(move_cursor) = self.project.integrate(msg) else {
            return;
        };

        let cursor = self
            .cursors
            .get_remote_mut(move_cursor.id())
            .expect("already received cursor creation");

        cursor
            .move_to(move_cursor)
            .expect("move op was applied to the correct cursor");

        if let Some(tooltip) = self.cursor_tooltips.get_mut(&cursor.id()) {
            let new_offset = self
                .project
                .file(cursor.file_id())
                .expect("move op was integrated, so file must still exist")
                .resolve_anchor(cursor.anchor());

            if let Some(new_offset) = new_offset {
                self.editor
                    .move_tooltip(&mut tooltip.inner, new_offset.into());
            }
        }
    }

    async fn integrate_moved_directory(
        &mut self,
        _msg: actions::move_directory::MovedDirectoryRemote,
    ) -> Result<(), RunSessionError> {
        Ok(())
    }

    async fn integrate_message(
        &mut self,
        msg: Message,
    ) -> Result<(), RunSessionError> {
        match msg {
            Message::PeerDisconnected(peer_id) => {
                self.on_peer_disconnected(peer_id);
                Ok(())
            },
            Message::Project(project_msg) => {
                self.integrate_project_message(project_msg).await
            },
            _ => {
                error!("received unexpected message: {msg:?}");
                Ok(())
            },
        }
    }

    async fn integrate_moved_file(
        &mut self,
        _msg: actions::move_file::MovedFileRemote,
    ) -> Result<(), RunSessionError> {
        Ok(())
    }

    async fn integrate_moved_selection(
        &mut self,
        _msg: actions::move_selection::MovedSelectionRemote,
    ) -> Result<(), RunSessionError> {
        Ok(())
    }

    async fn integrate_project_message(
        &mut self,
        msg: ProjectMessage,
    ) -> Result<(), RunSessionError> {
        match msg {
            ProjectMessage::CreatedCursor(msg) => {
                self.integrate_created_cursor(msg);
                Ok(())
            },
            ProjectMessage::CreatedDirectory(msg) => {
                self.integrate_created_directory(msg).await
            },
            ProjectMessage::CreatedFile(msg) => {
                self.integrate_created_file(msg).await
            },
            ProjectMessage::CreatedSelection(msg) => {
                self.integrate_created_selection(msg);
                Ok(())
            },
            ProjectMessage::DeletedText(msg) => {
                self.integrate_deleted_text(msg).await
            },
            ProjectMessage::InsertedText(msg) => {
                self.integrate_inserted_text(msg).await
            },
            ProjectMessage::MovedCursor(msg) => {
                self.integrate_moved_cursor(msg);
                Ok(())
            },
            ProjectMessage::MovedDirectory(msg) => {
                self.integrate_moved_directory(msg).await
            },
            ProjectMessage::MovedFile(msg) => {
                self.integrate_moved_file(msg).await
            },
            ProjectMessage::MovedSelection(msg) => {
                self.integrate_moved_selection(msg).await
            },
            ProjectMessage::RemovedCursor(msg) => {
                self.integrate_removed_cursor(msg);
                Ok(())
            },
            ProjectMessage::RemovedDirectory(msg) => {
                self.integrate_removed_directory(msg).await
            },
            ProjectMessage::RemovedFile(msg) => {
                self.integrate_removed_file(msg).await
            },
            ProjectMessage::RemovedSelection(msg) => {
                self.integrate_removed_selection(msg).await
            },
        }
    }

    fn integrate_removed_cursor(
        &mut self,
        msg: actions::remove_cursor::RemovedCursorRemote,
    ) {
        let Some(remove_cursor) = self.project.integrate(msg) else {
            return;
        };

        let cursor = self
            .cursors
            .remove_remote(remove_cursor.id())
            .expect("already received cursor creation");

        if let Some(tooltip) = self.cursor_tooltips.remove(&cursor.id()) {
            self.editor.remove_tooltip(tooltip.inner);
        }

        cursor
            .remove(remove_cursor)
            .expect("remove op was applied to the right cursor");
    }

    async fn integrate_removed_directory(
        &mut self,
        _msg: actions::remove_directory::RemovedDirectoryRemote,
    ) -> Result<(), RunSessionError> {
        Ok(())
    }

    async fn integrate_removed_file(
        &mut self,
        _msg: actions::remove_file::RemovedFileRemote,
    ) -> Result<(), RunSessionError> {
        Ok(())
    }

    async fn integrate_removed_selection(
        &mut self,
        _msg: actions::remove_selection::RemovedSelectionRemote,
    ) -> Result<(), RunSessionError> {
        Ok(())
    }

    fn is_ignored(&mut self, id: &E::FileId) -> bool {
        !self.editor.is_text_file(id)
    }

    fn is_in_project_tree(&mut self, id: &E::FileId) -> bool {
        let mut path = self.editor.path(id).into_owned();

        loop {
            if path == self.project_root {
                return true;
            }

            if !path.pop() {
                return false;
            }
        }
    }

    fn is_tracked(&self, file_id: &E::FileId) -> bool {
        self.subs_edits.contains_key(file_id)
    }

    fn on_closed_file(&mut self, file_id: E::FileId) {
        if self.is_tracked(&file_id) {
            self.subs_edits.remove(&file_id);
            self.subs_cursors.remove(&file_id);
            self.subs_selections.remove(&file_id);
        }
    }

    fn on_opened_file(&mut self, file_id: E::FileId) {
        assert!(!self.is_tracked(&file_id), "file already tracked");

        if self.is_in_project_tree(&file_id) && !self.is_ignored(&file_id) {
            let edits = self.editor.edits(&file_id);
            let cursors = self.editor.cursors(&file_id);
            let selections = self.editor.selections(&file_id);
            self.subs_edits.insert(file_id.clone(), edits);
            self.subs_cursors.insert(file_id.clone(), cursors);
            self.subs_selections.insert(file_id.clone(), selections);
        }
    }

    fn on_peer_disconnected(&mut self, peer_id: PeerId) {
        todo!();
    }

    async fn sync_cursor(
        &mut self,
        cursor: Cursor<E>,
    ) -> Result<(), RunSessionError> {
        match cursor.action {
            CursorAction::Created(offset) => {
                self.sync_created_cursor(
                    cursor.cursor_id,
                    cursor.file_id,
                    offset,
                )
                .await
            },
            CursorAction::Moved(offset) => {
                self.sync_moved_cursor(
                    cursor.cursor_id,
                    cursor.file_id,
                    offset,
                )
                .await
            },
            CursorAction::Removed => {
                self.sync_removed_cursor(cursor.cursor_id).await
            },
        }
    }

    async fn sync_created_cursor(
        &mut self,
        cursor_id: E::CursorId,
        file_id: E::FileId,
        offset: ByteOffset,
    ) -> Result<(), RunSessionError> {
        let file_id = self.to_project_file_id(file_id);

        let anchor = self
            .project
            .file(file_id)
            .expect("")
            .create_anchor(offset.into(), AnchorBias::Right);

        let action = actions::create_cursor::CreatedCursor { file_id, anchor };

        let (cursor, msg) = match self.project.synchronize(action) {
            Ok(res) => res,
            Err(err) => {
                error!("moved cursor to a deleted file: {err}");
                return Ok(());
            },
        };

        self.cursors.insert_local(cursor_id, cursor);

        self.broadcast(Message::Project(ProjectMessage::CreatedCursor(msg)))
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn sync_created_selection(
        &mut self,
        selection_id: E::SelectionId,
        file_id: E::FileId,
        head: ByteOffset,
        tail: ByteOffset,
    ) -> Result<(), RunSessionError> {
        let file_id = self.to_project_file_id(file_id);
        let file = self.project.file(file_id).expect("");

        let head_bias =
            if head < tail { AnchorBias::Right } else { AnchorBias::Left };

        let action = actions::create_selection::CreatedSelection {
            file_id,
            head: file.create_anchor(head.into(), head_bias),
            tail: file.create_anchor(tail.into(), !head_bias),
        };

        let (selection, msg) = match self.project.synchronize(action) {
            Ok(res) => res,
            Err(err) => {
                error!("moved selection to a deleted file: {err}");
                return Ok(());
            },
        };

        self.selections.insert_local(selection_id, selection);

        self.broadcast(Message::Project(ProjectMessage::CreatedSelection(msg)))
            .await
    }

    async fn sync_edit(
        &mut self,
        edit: Edit<E>,
    ) -> Result<(), RunSessionError> {
        let file_id = self.to_project_file_id(edit.file_id);

        for hunk in edit.hunks {
            let byte_range = hunk.deleted_byte_range();

            if !byte_range.is_empty() {
                let action =
                    actions::delete_text::DeletedText { file_id, byte_range };

                let msg = match self.project.synchronize(action) {
                    Ok(msg) => msg,
                    Err(err) => {
                        error!("deleted text from a deleted file: {err}");
                        continue;
                    },
                };

                self.broadcast(Message::Project(ProjectMessage::DeletedText(
                    msg,
                )))
                .await?;
            }

            if !hunk.text.is_empty() {
                let action = actions::insert_text::InsertedText {
                    file_id,
                    byte_offset: hunk.start.into(),
                    text_len: hunk.text.len(),
                };

                let msg = match self.project.synchronize(action) {
                    Ok(msg) => msg,
                    Err(err) => {
                        error!("inserted text into a deleted file: {err}");
                        continue;
                    },
                };

                self.broadcast(Message::Project(
                    ProjectMessage::InsertedText(msg),
                ))
                .await?;
            }
        }

        Ok(())
    }

    async fn sync_moved_cursor(
        &mut self,
        cursor_id: E::CursorId,
        file_id: E::FileId,
        offset: ByteOffset,
    ) -> Result<(), RunSessionError> {
        let file_id = self.to_project_file_id(file_id);

        let anchor = self
            .project
            .file(file_id)
            .expect("")
            .create_anchor(offset.into(), AnchorBias::Right);

        let cursor = self
            .cursors
            .get_local_mut(cursor_id)
            .expect("already received its creation");

        let action = actions::move_cursor::MovedCursor { anchor, cursor };

        let msg = self.project.synchronize(action);

        self.broadcast(Message::Project(ProjectMessage::MovedCursor(msg)))
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn sync_moved_selection(
        &mut self,
        selection_id: E::SelectionId,
        file_id: E::FileId,
        head: ByteOffset,
        tail: ByteOffset,
    ) -> Result<(), RunSessionError> {
        let file_id = self.to_project_file_id(file_id);
        let file = self.project.file(file_id).expect("");

        let head_bias =
            if head < tail { AnchorBias::Right } else { AnchorBias::Left };

        let selection = self
            .selections
            .get_local_mut(selection_id)
            .expect("already received its creation");

        let action = actions::move_selection::MovedSelection {
            selection,
            head: file.create_anchor(head.into(), head_bias),
            tail: file.create_anchor(tail.into(), !head_bias),
        };

        let msg = self.project.synchronize(action);

        self.broadcast(Message::Project(ProjectMessage::MovedSelection(msg)))
            .await
    }

    async fn sync_removed_selection(
        &mut self,
        selection_id: E::SelectionId,
    ) -> Result<(), RunSessionError> {
        let selection = self
            .selections
            .remove_local(selection_id)
            .expect("selection has not been removed yet");

        let action = actions::remove_selection::RemovedSelection { selection };

        let msg = self.project.synchronize(action);

        self.broadcast(Message::Project(ProjectMessage::RemovedSelection(msg)))
            .await
    }

    async fn sync_removed_cursor(
        &mut self,
        cursor_id: E::CursorId,
    ) -> Result<(), RunSessionError> {
        let cursor = self
            .cursors
            .remove_local(cursor_id)
            .expect("cursor has not been removed yet");

        let action = actions::remove_cursor::RemovedCursor { cursor };

        let msg = self.project.synchronize(action);

        self.broadcast(Message::Project(ProjectMessage::RemovedCursor(msg)))
            .await
    }

    async fn sync_selection(
        &mut self,
        selection: Selection<E>,
    ) -> Result<(), RunSessionError> {
        match selection.action {
            SelectionAction::Created { head, tail } => {
                self.sync_created_selection(
                    selection.selection_id,
                    selection.file_id,
                    head,
                    tail,
                )
                .await
            },
            SelectionAction::Moved { head, tail } => {
                self.sync_moved_selection(
                    selection.selection_id,
                    selection.file_id,
                    head,
                    tail,
                )
                .await
            },
            SelectionAction::Removed => {
                self.sync_removed_selection(selection.selection_id).await
            },
        }
    }

    fn to_editor_file_id(&self, _file_id: FileId) -> Option<E::FileId> {
        todo!();
    }

    fn to_project_file_id(&self, _file_id: E::FileId) -> FileId {
        todo!();
    }
}

impl<E: CollabEditor> Default for Cursors<E> {
    fn default() -> Self {
        Self { local: Default::default(), remote: Default::default() }
    }
}

impl<E: CollabEditor> Cursors<E> {
    fn get_local_mut(
        &mut self,
        id: E::CursorId,
    ) -> Option<&mut cursor::Cursor> {
        self.local.get_mut(&id)
    }

    fn get_remote_mut(
        &mut self,
        id: cursor::CursorId,
    ) -> Option<&mut cursor::Cursor> {
        self.remote.get_mut(&id)
    }

    /// # Panics
    ///
    /// Panics if the given cursor ID already exists.
    #[track_caller]
    fn insert_local(&mut self, id: E::CursorId, cursor: cursor::Cursor) {
        if self.local.insert(id.clone(), cursor).is_some() {
            panic!("cursor {id:?} already exists");
        }
    }

    fn insert_remote(&mut self, cursor: cursor::Cursor) {
        if self.remote.insert(cursor.id(), cursor).is_some() {
            panic!("cursor already exists");
        }
    }

    fn remove_local(&mut self, id: E::CursorId) -> Option<cursor::Cursor> {
        self.local.remove(&id)
    }

    fn remove_remote(
        &mut self,
        id: cursor::CursorId,
    ) -> Option<cursor::Cursor> {
        self.remote.remove(&id)
    }
}

impl<E: CollabEditor> Default for Selections<E> {
    fn default() -> Self {
        Self { local: Default::default(), remote: Default::default() }
    }
}

impl<E: CollabEditor> Selections<E> {
    fn get_local_mut(
        &mut self,
        id: E::SelectionId,
    ) -> Option<&mut selection::Selection> {
        self.local.get_mut(&id)
    }

    fn get_remote_mut(
        &mut self,
        id: selection::SelectionId,
    ) -> Option<&mut selection::Selection> {
        self.remote.get_mut(&id)
    }

    /// # Panics
    ///
    /// Panics if the given selection ID already exists.
    #[track_caller]
    fn insert_local(
        &mut self,
        id: E::SelectionId,
        selection: selection::Selection,
    ) {
        if self.local.insert(id.clone(), selection).is_some() {
            panic!("selection {id:?} already exists");
        }
    }

    fn insert_remote(&mut self, selection: selection::Selection) {
        if self.remote.insert(selection.id(), selection).is_some() {
            panic!("selection already exists");
        }
    }

    fn remove_local(
        &mut self,
        id: E::SelectionId,
    ) -> Option<selection::Selection> {
        self.local.remove(&id)
    }

    fn remove_remote(
        &mut self,
        id: selection::SelectionId,
    ) -> Option<selection::Selection> {
        self.remote.remove(&id)
    }
}

async fn ask_for_project(
    joined: &mut Joined,
) -> Result<Project, JoinSessionError> {
    todo!();
    // let local_id = joined.join_response.client_id;
    //
    // let &ask_project_to =
    //     joined.peers.iter().find(|id| id != local_id).expect("never empty");
    //
    // let message = Message::ProjectRequest(local_id);
    //
    // let outbound = Outbound {
    //     should_compress: message.should_compress(),
    //     message,
    //     recipients: Recipients::only([ask_project_to]),
    // };
    //
    // let mut buffered = Vec::new();
    //
    // let mut project = loop {
    //     let message = match this.receiver.next().await {
    //         Some(Ok(message)) => message,
    //         Some(Err(err)) => return Err(err.into()),
    //         None => todo!(),
    //     };
    //
    //     match message {
    //         Message::ProjectResponse(project) => break project,
    //         other => buffered.push(other),
    //     }
    // };
    //
    // for project_msg in buffered {
    //     let _ = project.integrate(project_msg);
    // }
    //
    // project
}

async fn create_project_dir(
    project: &Project,
    project_root: &AbsUtf8Path,
    fs: &mut impl Fs,
) -> Result<(), JoinSessionError> {
    fs.create_dir(project_root).await?;
    fs.set_root(project_root.to_owned());
    Ok(())
}

impl<E: CollabEditor> Drop for Session<E> {
    fn drop(&mut self) {
        if self.is_host() {
            return;
        }

        // let fs = self.ctx.fs();
        // let project_root = self.project_root.clone();
        //
        // self.ctx
        //     .spawner()
        //     .spawn(async move {
        //         if let Err(err) = fs.remove_dir(&project_root).await {
        //             println!("failed to remove project directory: {err}");
        //         }
        //     })
        //     .detach();
    }
}

struct CursorTooltip<E: CollabEditor> {
    cursor_id: cursor::CursorId,
    inner: E::Tooltip,
}

struct SelectionHighlight<E: CollabEditor> {
    selection_id: selection::SelectionId,
    inner: E::Highlight,
}

struct ConfirmStart<'path>(&'path AbsUtf8Path);

impl fmt::Display for ConfirmStart<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "found root of project at '{}'. Start session?", self.0)
    }
}

#[derive(Debug)]
pub(crate) enum JoinSessionError {}

impl From<io::Error> for JoinSessionError {
    fn from(err: io::Error) -> Self {
        todo!();
    }
}

#[derive(Debug)]
pub(crate) enum RunSessionError {}

impl From<io::Error> for RunSessionError {
    fn from(err: io::Error) -> Self {
        todo!();
    }
}

#[derive(Debug)]
pub(crate) enum StartSessionError {
    /// The session was started in a non-file buffer.
    NotInFile,

    /// It was not possible to find the root of the project containing the
    /// file at the given path.
    CouldntFindRoot(AbsUtf8PathBuf),

    /// We asked the user for confirmation to start the session, but they
    /// cancelled.
    UserCancelled,
}

impl From<io::Error> for StartSessionError {
    fn from(err: io::Error) -> Self {
        todo!();
    }
}
