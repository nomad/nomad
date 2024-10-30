mod detach_buffer_actions;
mod peer_selection;
mod peer_tooltip;
mod project;
mod register_buffer_actions;
mod sync_cursor;
mod sync_replacement;

use core::future::Future;
use std::io;

use collab_server::message::{
    Message,
    Peer,
    Peers,
    Project as CollabProject,
    ProjectRequest,
    ProjectResponse,
    ProjectTree,
};
use detach_buffer_actions::DetachBufferActions;
use e31e::fs::AbsPath;
use futures_util::{
    pin_mut,
    select,
    FutureExt,
    Sink,
    SinkExt,
    Stream,
    StreamExt,
};
use nomad::autocmds::{BufAdd, BufUnload};
use nomad::ctx::NeovimCtx;
use nomad::diagnostics::{
    DiagnosticMessage,
    DiagnosticSource,
    HighlightGroup,
    Level,
};
use nomad::{BufferId, Event, Module, Shared};
use peer_selection::PeerSelection;
use peer_tooltip::PeerTooltip;
pub(crate) use project::Project;
use register_buffer_actions::RegisterBufferActions;
use sync_cursor::SyncCursor;
use sync_replacement::SyncReplacement;
use tracing::error;

use crate::Collab;

/// TODO: docs.
#[derive(Clone)]
pub(crate) struct Session {
    neovim_ctx: NeovimCtx<'static>,
    project: Shared<Project>,
}

impl Session {
    pub(crate) async fn join() -> Self {
        todo!();
    }

    pub(crate) async fn start() -> Self {
        todo!();
    }

    pub(crate) async fn run<Tx, Rx>(&mut self, remote_tx: Tx, remote_rx: Rx)
    where
        Tx: Sink<Message, Error = core::convert::Infallible>,
        Rx: Stream<Item = Message>,
    {
        let (local_tx, local_rx) = flume::unbounded();

        let mut register_buffer_actions = RegisterBufferActions {
            message_tx: local_tx.clone(),
            project: self.project.clone(),
        };

        let detach_buffer_actions = DetachBufferActions {
            message_tx: local_tx.clone(),
            project: self.project.clone(),
        };

        for buffer_id in BufferId::opened() {
            register_buffer_actions.register_actions(buffer_id);
        }

        BufAdd::new(register_buffer_actions)
            .register(self.neovim_ctx.reborrow());

        BufUnload::new(detach_buffer_actions)
            .register(self.neovim_ctx.reborrow());

        pin_mut!(remote_rx);
        pin_mut!(remote_tx);

        loop {
            select! {
                remote_message = remote_rx.next().fuse() => {
                    if let Some(remote_message) = remote_message {
                        self.integrate_message(remote_message, &local_tx);
                    }
                },
                local_message = local_rx.recv_async().fuse() => {
                    if let Ok(local_message) = local_message {
                        remote_tx
                            .send(local_message)
                            .await
                            .expect("Infallible");
                    }
                },
            }
        }
    }

    fn integrate_message(
        &self,
        message: Message,
        message_tx: &flume::Sender<Message>,
    ) {
        use Message::*;
        match message {
            CreatedCursor(msg) => self.integrate_created_cursor(msg),
            CreatedDirectory(msg) => self.integrate_created_directory(msg),
            CreatedFile(msg) => self.integrate_created_file(msg),
            CreatedSelection(msg) => self.integrate_created_selection(msg),
            EditedBuffer(msg) => self.integrate_edited_buffer(msg),
            MovedCursor(msg) => self.integrate_moved_cursor(msg),
            MovedDirectory(msg) => self.integrate_moved_directory(msg),
            MovedFile(msg) => self.integrate_moved_file(msg),
            MovedSelection(msg) => self.integrate_moved_selection(msg),
            PeerDisconnected(msg) => self.integrate_peer_disconnected(msg),
            PeerJoined(msg) => self.integrate_peer_joined(msg),
            PeerLeft(msg) => self.integrate_peer_left(msg),
            RemovedCursor(msg) => self.integrate_removed_cursor(msg),
            RemovedSelection(msg) => self.integrate_removed_selection(msg),
            RemovedFile(msg) => self.integrate_removed_file(msg),
            RemovedDirectory(msg) => self.integrate_removed_directory(msg),
            ProjectRequest(msg) => {
                self.handle_project_request(msg, message_tx.clone())
            },
            ProjectResponse(msg) => {
                error!("received unexpected ProjectResponse: {:?}", msg)
            },
        }
    }

    fn integrate_created_cursor(&self, cursor_creation: e31e::CursorCreation) {
        self.project
            .with_mut(|p| p.integrate_cursor_creation(cursor_creation));
    }

    fn integrate_created_directory(
        &self,
        directory_creation: e31e::DirectoryCreation,
    ) {
        self.project.with_mut(|p| {
            if let Some(_create_directory) =
                p.replica.integrate_directory_creation(directory_creation)
            {
                todo!();
            }
        });
    }

    fn integrate_created_file(&self, file_creation: e31e::FileCreation) {
        let Some((_file_path, _replacements)) = self.project.with_mut(|p| {
            p.replica.integrate_file_creation(file_creation).map(
                |create_file| (create_file.file.path(), create_file.hunks),
            )
        }) else {
            return;
        };
    }

    fn integrate_created_selection(
        &self,
        selection_creation: e31e::SelectionCreation,
    ) {
        self.project
            .with_mut(|p| p.integrate_selection_creation(selection_creation));
    }

    fn integrate_edited_buffer(&self, edit: e31e::Edit) {
        if let Some((_file_path, _replacements)) =
            self.project.with_mut(|p| p.integrate_edit(edit))
        {
            todo!();
        }
    }

    fn integrate_moved_cursor(
        &self,
        cursor_relocation: e31e::CursorRelocation,
    ) {
        self.project
            .with_mut(|p| p.integrate_cursor_relocation(cursor_relocation));
    }

    fn integrate_moved_directory(
        &self,
        directory_relocation: e31e::DirectoryRelocation,
    ) {
        if let Some((_old_path, _new_path)) = self.project.with_mut(|p| {
            p.replica.integrate_directory_relocation(directory_relocation).map(
                |relocate_dir| {
                    let dir = &relocate_dir.directory;
                    let mut new_path = dir.path();
                    new_path.push(dir.name().expect("can't be root"));
                    (relocate_dir.old_path, new_path)
                },
            )
        }) {
            todo!();
        }
    }

    fn integrate_moved_file(&self, file_relocation: e31e::FileRelocation) {
        if let Some((_old_path, _new_path)) = self.project.with_mut(|p| {
            p.replica.integrate_file_relocation(file_relocation).map(
                |relocate_file| {
                    let file = &relocate_file.file;
                    let mut new_path = file.path();
                    new_path.push(file.name());
                    (relocate_file.old_path, new_path)
                },
            )
        }) {
            todo!();
        }
    }

    fn integrate_moved_selection(
        &self,
        selection_relocation: e31e::SelectionRelocation,
    ) {
        self.project.with_mut(|p| {
            p.integrate_selection_relocation(selection_relocation)
        });
    }

    fn integrate_peer_disconnected(&self, peer_id: e31e::PeerId) {
        self.project.with_mut(|p| p.integrate_peer_left(peer_id));
    }

    fn integrate_peer_joined(&self, peer: Peer) {
        self.project.with_mut(|p| p.integrate_peer_joined(peer));
    }

    fn integrate_peer_left(&self, peer_id: e31e::PeerId) {
        self.project.with_mut(|p| p.integrate_peer_left(peer_id));
    }

    fn integrate_removed_cursor(&self, cursor_removal: e31e::CursorRemoval) {
        self.project.with_mut(|p| p.integrate_cursor_removal(cursor_removal));
    }

    fn integrate_removed_selection(
        &self,
        selection_removal: e31e::SelectionRemoval,
    ) {
        self.project
            .with_mut(|p| p.integrate_selection_removal(selection_removal));
    }

    fn integrate_removed_file(&self, file_removal: e31e::FileRemoval) {
        if let Some(_remove_file) = self
            .project
            .with_mut(|p| p.replica.integrate_file_removal(file_removal))
        {
            todo!();
        }
    }

    fn integrate_removed_directory(
        &self,
        directory_removal: e31e::DirectoryRemoval,
    ) {
        if let Some(_remove_directory) = self.project.with_mut(|p| {
            p.replica.integrate_directory_removal(directory_removal)
        }) {
            todo!();
        }
    }

    fn handle_project_request(
        &self,
        project_request: ProjectRequest,
        message_tx: flume::Sender<Message>,
    ) {
        self.neovim_ctx.spawn({
            let this = self.clone();
            let respond_to = project_request.request_from;
            async move {
                match this.read_project().await {
                    Ok(project) => {
                        let _ = message_tx.send(Message::ProjectResponse(
                            Box::new(ProjectResponse {
                                respond_to: respond_to.id(),
                                project,
                            }),
                        ));
                    },
                    Err(err) => {
                        ReadProjectError { err, respond_to }.emit();
                    },
                };
            }
        });
    }

    async fn read_project(&self) -> io::Result<CollabProject> {
        let (peers, replica) = self.project.with(|p| {
            (p.all_peers().cloned().collect::<Peers>(), p.replica.clone())
        });

        match ProjectTree::new(replica, |file_path| self.read_file(file_path))
            .await
        {
            Ok(tree) => Ok(CollabProject { peers, tree }),
            Err((err, _)) => Err(err),
        }
    }

    fn read_file<'a>(
        &'a self,
        _file_path: &AbsPath,
    ) -> impl Future<Output = std::io::Result<Box<str>>> + 'a {
        async move { Ok("".into()) }
    }
}

struct ReadProjectError {
    err: io::Error,
    respond_to: Peer,
}

impl ReadProjectError {
    fn emit(&self) {
        self.message().emit(Level::Error, self.source());
    }

    fn message(&self) -> DiagnosticMessage {
        let mut message = DiagnosticMessage::new();
        message
            .push_str("couldn't send project to ")
            .push_str_highlighted(
                self.respond_to.github_handle().to_string(),
                HighlightGroup::special(),
            )
            .push_str(": ")
            .push_str(self.err.to_string());
        message
    }

    fn source(&self) -> DiagnosticSource {
        let mut source = DiagnosticSource::new();
        source.push_segment(<Collab as Module>::NAME.as_str());
        source
    }
}
