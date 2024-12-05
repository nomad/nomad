mod detach_buffer_actions;
mod peer_selection;
mod peer_tooltip;
mod project;
mod register_buffer_actions;
mod sync_cursor;
mod sync_replacement;

use std::io;

use collab_server::message::{
    FileContents,
    Message,
    Peer,
    Peers,
    ProjectRequest,
    ProjectResponse,
};
use collab_server::SessionId;
use detach_buffer_actions::DetachBufferActions;
use eerie::fs::AbsPathBuf;
use futures_util::{
    pin_mut,
    select,
    stream,
    FutureExt,
    Sink,
    SinkExt,
    Stream,
    StreamExt,
};
use nvimx::ctx::{BufferId, NeovimCtx};
use nvimx::diagnostics::{
    DiagnosticMessage,
    DiagnosticSource,
    HighlightGroup,
    Level,
};
use nvimx::event::{BufAdd, BufUnload, Event};
use nvimx::plugin::Module;
use nvimx::Shared;
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

pub(crate) struct NewSessionArgs {
    /// Whether the [`local_peer`](Self::local_peer) is the host of the
    /// session.
    pub(crate) is_host: bool,

    /// The local [`Peer`].
    pub(crate) local_peer: Peer,

    /// The remote [`Peers`].
    pub(crate) remote_peers: Peers,

    /// The absolute path to the directory containing the project.
    ///
    /// The contents of the directory are assumed to be in sync with with the
    /// [`replica`](Self::replica).
    pub(crate) project_root: AbsPathBuf,

    /// The [`replica`](Self::replica) of the project.
    ///
    /// The files and directories in it are assumed to be in sync with the
    /// contents of the [`project_root`](Self::project_root).
    pub(crate) replica: eerie::Replica,

    /// The ID of the session.
    pub(crate) session_id: SessionId,

    /// An instance of the [`NeovimCtx`].
    pub(crate) neovim_ctx: NeovimCtx<'static>,
}

impl Session {
    pub(crate) fn new(args: NewSessionArgs) -> Self {
        let project = Project {
            actor_id: args.neovim_ctx.next_actor_id(),
            buffer_actions: Default::default(),
            local_cursor_id: None,
            local_peer: args.local_peer,
            neovim_ctx: args.neovim_ctx.clone(),
            project_root: args.project_root,
            remote_peers: args
                .remote_peers
                .into_iter()
                .map(|peer| (peer.id(), peer))
                .collect(),
            remote_selections: Default::default(),
            remote_tooltips: Default::default(),
            replica: args.replica,
            session_id: args.session_id,
        };
        Self { neovim_ctx: args.neovim_ctx, project: Shared::new(project) }
    }

    pub(crate) fn project(&self) -> Shared<Project> {
        self.project.clone()
    }

    pub(crate) async fn run<Tx, Rx, RxError>(
        &self,
        remote_tx: Tx,
        remote_rx: Rx,
    ) -> Result<(), RunSessionError<Tx::Error, RxError>>
    where
        Tx: Sink<Message>,
        Rx: Stream<Item = Result<Message, RxError>>,
    {
        let (local_tx, local_rx) = flume::unbounded();

        let mut register_buffer_actions = RegisterBufferActions {
            message_tx: local_tx.clone(),
            project: self.project.clone(),
        };

        let detach_buffer_actions =
            DetachBufferActions { project: self.project.clone() };

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
                msg = remote_rx.next().fuse() => {
                    let Some(msg_res) = msg else { continue };
                    let remote_message = msg_res.map_err(RunSessionError::Receive)?;
                    self.integrate_message(remote_message, &local_tx);
                },
                msg = local_rx.recv_async().fuse() => {
                    if let Ok(local_message) = msg {
                        remote_tx
                            .send(local_message)
                            .await
                            .map_err(RunSessionError::Send)?;
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

    fn integrate_created_cursor(
        &self,
        cursor_creation: eerie::CursorCreation,
    ) {
        self.project
            .with_mut(|p| p.integrate_cursor_creation(cursor_creation));
    }

    fn integrate_created_directory(
        &self,
        directory_creation: eerie::DirectoryCreation,
    ) {
        self.project.with_mut(|p| {
            if let Some(_create_directory) =
                p.replica.integrate_directory_creation(directory_creation)
            {
                todo!();
            }
        });
    }

    fn integrate_created_file(&self, file_creation: eerie::FileCreation) {
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
        selection_creation: eerie::SelectionCreation,
    ) {
        self.project
            .with_mut(|p| p.integrate_selection_creation(selection_creation));
    }

    fn integrate_edited_buffer(&self, edit: eerie::Edit) {
        if let Some((_file_path, _replacements)) =
            self.project.with_mut(|p| p.integrate_edit(edit))
        {
            todo!();
        }
    }

    fn integrate_moved_cursor(
        &self,
        cursor_relocation: eerie::CursorRelocation,
    ) {
        self.project
            .with_mut(|p| p.integrate_cursor_relocation(cursor_relocation));
    }

    fn integrate_moved_directory(
        &self,
        directory_relocation: eerie::DirectoryRelocation,
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

    fn integrate_moved_file(&self, file_relocation: eerie::FileRelocation) {
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
        selection_relocation: eerie::SelectionRelocation,
    ) {
        self.project.with_mut(|p| {
            p.integrate_selection_relocation(selection_relocation)
        });
    }

    fn integrate_peer_disconnected(&self, peer_id: eerie::PeerId) {
        self.project.with_mut(|p| p.integrate_peer_left(peer_id));
    }

    fn integrate_peer_joined(&self, peer: Peer) {
        self.project.with_mut(|p| p.integrate_peer_joined(peer));
    }

    fn integrate_peer_left(&self, peer_id: eerie::PeerId) {
        self.project.with_mut(|p| p.integrate_peer_left(peer_id));
    }

    fn integrate_removed_cursor(&self, cursor_removal: eerie::CursorRemoval) {
        self.project.with_mut(|p| p.integrate_cursor_removal(cursor_removal));
    }

    fn integrate_removed_selection(
        &self,
        selection_removal: eerie::SelectionRemoval,
    ) {
        self.project
            .with_mut(|p| p.integrate_selection_removal(selection_removal));
    }

    fn integrate_removed_file(&self, file_removal: eerie::FileRemoval) {
        if let Some(_remove_file) = self
            .project
            .with_mut(|p| p.replica.integrate_file_removal(file_removal))
        {
            todo!();
        }
    }

    fn integrate_removed_directory(
        &self,
        directory_removal: eerie::DirectoryRemoval,
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
        self.neovim_ctx.spawn(|_| {
            let this = self.clone();
            let respond_to = project_request.requested_by;
            async move {
                let contents = match this.read_file_contents().await {
                    Ok(contents) => contents,
                    Err(err) => {
                        ReadProjectError { err, respond_to }.emit();
                        return;
                    },
                };
                let response = this.project.with(move |p| ProjectResponse {
                    respond_to: respond_to.id(),
                    file_contents: contents,
                    peers: p.all_peers().cloned().collect(),
                    replica: p.replica.encode(),
                });
                let _ = message_tx
                    .send(Message::ProjectResponse(Box::new(response)));
            }
        });
    }

    async fn read_file_contents(&self) -> io::Result<FileContents> {
        let project_root = self.project.with(|p| p.project_root.clone());
        let mut read_files = self.project.with(|p| {
            p.replica
                .files()
                .map(|file| {
                    let file_id = file.id();
                    let mut file_path = project_root.clone();
                    file_path.concat(&file.path());
                    async move {
                        let contents =
                            async_fs::read_to_string(file_path).await?;
                        Ok::<_, io::Error>((
                            file_id,
                            contents.into_boxed_str(),
                        ))
                    }
                })
                .collect::<stream::FuturesUnordered<_>>()
        });
        let mut file_contents = FileContents::new();
        while let Some(result) = read_files.next().await {
            let (file_id, contents) = result?;
            file_contents.set(file_id, contents);
        }
        Ok(file_contents)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RunSessionError<TxErr, RxErr> {
    #[error("failed to send message: {0}")]
    Send(TxErr),
    #[error("failed to receive message: {0}")]
    Receive(RxErr),
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
