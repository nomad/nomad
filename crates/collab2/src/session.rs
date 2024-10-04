use core::fmt;
use std::collections::HashMap;
use std::io;

use collab_fs::{AbsUtf8Path, AbsUtf8PathBuf, Fs};
use collab_messaging::{Outbound, PeerId, Recipients};
use collab_project::file::FileId;
use collab_project::{Integrate, Project, Synchronize};
use collab_server::JoinRequest;
use futures_util::stream::select_all;
use futures_util::{select, FutureExt, StreamExt};
use nohash::IntSet as NoHashSet;
use nomad::{Context, Editor, Event, JoinHandle, Spawner, Subscription};
use nomad_server::client::{Joined, Receiver, Sender};
use nomad_server::{Io, Message};
use root_finder::markers::Git;
use root_finder::Finder;

use crate::events::{
    Cursor,
    CursorEvent,
    Edit,
    EditEvent,
    Selection,
    SelectionEvent,
};
use crate::{CollabEditor, Config, SessionId};

pub(crate) struct Session<E: CollabEditor> {
    /// TODO: docs.
    config: Config,

    /// TODO: docs.
    ctx: Context<E>,

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
    subs_edits: HashMap<FileId, E::EditStream>,

    /// TODO: docs.
    subs_cursors: HashMap<FileId, E::CursorStream>,

    /// TODO: docs.
    subs_selections: HashMap<FileId, E::SelectionStream>,
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
        Self {
            config,
            ctx,
            id: SessionId(join_response.session_id),
            peers,
            project,
            project_root,
            receiver,
            sender,
            server_id: join_response.server_id,
            subs_cursors: HashMap::new(),
            subs_edits: HashMap::new(),
            subs_selections: HashMap::new(),
        }
    }
}

impl<E: CollabEditor> Session<E> {
    async fn integrate_message(
        &mut self,
        message: Message,
    ) -> Result<(), RunSessionError> {
        todo!();
    }

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
                    self.sync_cursor_moved(cursor).await?;
                },

                edit = edits.next().fuse() => {
                    let edit = edit.expect("never ends");
                    drop(cursors);
                    drop(edits);
                    drop(selections);
                    self.sync_local_edit(edit).await?;
                },

                selection = selections.next().fuse() => {
                    let selection = selection.expect("never ends");
                    drop(cursors);
                    drop(edits);
                    drop(selections);
                    self.sync_selection_changed(selection).await?;
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

    #[inline]
    async fn sync_cursor_moved(
        &mut self,
        cursor: Cursor,
    ) -> Result<(), RunSessionError> {
        todo!();
    }

    #[inline]
    async fn sync_local_edit(
        &mut self,
        edit: Edit,
    ) -> Result<(), RunSessionError> {
        todo!();
    }

    #[inline]
    async fn sync_selection_changed(
        &mut self,
        selection: Selection,
    ) -> Result<(), RunSessionError> {
        todo!();
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

        let fs = self.ctx.fs();
        let project_root = self.project_root.clone();

        self.ctx
            .spawner()
            .spawn(async move {
                if let Err(err) = fs.remove_dir(&project_root).await {
                    println!("failed to remove project directory: {err}");
                }
            })
            .detach();
    }
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
