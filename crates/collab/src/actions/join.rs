use std::io;
use std::rc::Rc;

use collab_server::message::{
    FileContents,
    GitHubHandle,
    Message,
    Peer,
    Peers,
    ProjectRequest,
    ProjectResponse,
};
use collab_server::AuthInfos;
use e31e::fs::AbsPathBuf;
use e31e::{DirectoryId, DirectoryRef, Replica};
use futures_util::{
    future,
    stream,
    AsyncWriteExt,
    SinkExt,
    Stream,
    StreamExt,
};
use nvimx::ctx::NeovimCtx;
use nvimx::diagnostics::DiagnosticMessage;
use nvimx::plugin::{action_name, ActionName, AsyncAction};
use nvimx::Shared;

use super::UserBusyError;
use crate::session::{NewSessionArgs, RunSessionError, Session};
use crate::session_id::SessionId;
use crate::session_status::SessionStatus;
use crate::Collab;

#[derive(Clone)]
pub(crate) struct Join {
    session_status: Shared<SessionStatus>,
}

impl Join {
    pub(crate) fn new(session_status: Shared<SessionStatus>) -> Self {
        Self { session_status }
    }
}

impl AsyncAction for Join {
    const NAME: ActionName = action_name!("join");
    type Args = SessionId;
    type Docs = ();
    type Module = Collab;

    async fn execute(
        &mut self,
        session_id: Self::Args,
        ctx: NeovimCtx<'_>,
    ) -> Result<(), JoinError> {
        let auth_infos = AuthInfos {
            github_handle: "noib3"
                .parse::<GitHubHandle>()
                .expect("it's valid"),
        };

        let guard = JoinGuard::new(self.session_status.clone(), session_id)?;

        todo!();
        // #[rustfmt::skip]
        // let (step, maybe_err) = ConnectToServer { guard }
        //     .connect_to_server().emitting(spin::<ConnectToServer>()).await?
        //     .authenticate(auth_infos).emitting(spin::<Authenticate>()).await?
        //     .join_session(session_id).emitting(spin::<JoinSession>()).await?
        //     .confirm_join().await?
        //     .request_project().emitting(spin::<RequestProject>()).await?
        //     .find_project_root().emitting(spin::<FindProjectRoot>()).await?
        //     .flush_project(ctx).emitting(spin::<FlushProject>()).await?
        //     .jump_to_host().map_emit_by_ref(JoinedProject)
        //     .run_session(ctx.to_static()).await;
        //
        // step.remove_project_root()
        //     .await
        //     .map_err(Into::into)
        //     .map_err(|err| maybe_err.map(Into::into).unwrap_or(err))
    }

    fn docs(&self) {}
}

// 1: add a new crate that defines Emit and EmitExt traits;
// 2: crate depends on Action and DiagnosticMessage;
// 3:

struct JoinGuard {
    session_status: Shared<SessionStatus>,
}

struct ConnectToServer {
    guard: JoinGuard,
}

struct Authenticate {
    io: collab_server::Io,
    guard: JoinGuard,
}

struct JoinSession {
    authenticated: collab_server::client::Authenticated,
    auth_infos: AuthInfos,
    guard: JoinGuard,
}

struct ConfirmJoin {
    joined: collab_server::client::Joined,
    local_peer: Peer,
    guard: JoinGuard,
}

struct RequestProject {
    joined: collab_server::client::Joined,
    local_peer: Peer,
    guard: JoinGuard,
}

struct FindProjectRoot {
    buffered: Vec<Message>,
    local_peer: Peer,
    joined: collab_server::client::Joined,
    project_response: Box<ProjectResponse>,
    guard: JoinGuard,
}

struct FlushProject {
    buffered: Vec<Message>,
    local_peer: Peer,
    joined: collab_server::client::Joined,
    project_response: Box<ProjectResponse>,
    project_root: AbsPathBuf,
    guard: JoinGuard,
}

struct JumpToHost {
    buffered: Vec<Message>,
    joined: collab_server::client::Joined,
    local_peer: Peer,
    project_root: AbsPathBuf,
    remote_peers: Peers,
    replica: Replica,
    guard: JoinGuard,
}

struct RemoveProjectRoot {
    project_root: AbsPathBuf,
}

struct RunSession {
    buffered: Vec<Message>,
    joined: collab_server::client::Joined,
    local_peer: Peer,
    project_root: AbsPathBuf,
    remote_peers: Peers,
    replica: Replica,
    guard: JoinGuard,
}

struct JoinedProject<'a>(&'a RunSession);

#[derive(Debug, thiserror::Error)]
pub(crate) enum JoinError {
    #[error(transparent)]
    ConnectToServer(#[from] ConnectToServerError),

    #[error(transparent)]
    Authenticate(#[from] collab_server::client::AuthError),

    #[error(transparent)]
    JoinSession(#[from] JoinSessionError),

    #[error(transparent)]
    ConfirmJoin(#[from] ConfirmJoinError),

    #[error(transparent)]
    RequestProject(#[from] RequestProjectError),

    #[error(transparent)]
    FindProjectRoot(#[from] FindProjectRootError),

    #[error(transparent)]
    FlushProject(#[from] FlushProjectError),

    #[error(transparent)]
    JumpToHost(#[from] JumpToHostError),

    #[error(transparent)]
    RunSession(#[from] RunSessionError<io::Error, io::Error>),

    #[error(transparent)]
    RemoveProjectRoot(#[from] RemoveProjectRootError),

    #[error(transparent)]
    UserBusy(#[from] UserBusyError<false>),
}

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct ConnectToServerError {
    #[from]
    inner: io::Error,
}

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct JoinSessionError {
    #[from]
    inner: collab_server::client::JoinError,
}

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct ConfirmJoinError;

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) enum RequestProjectError {
    #[error("")]
    SendRequest(io::Error),

    #[error("")]
    ReadResponse(io::Error),
}

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct FindProjectRootError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum FlushProjectError {
    #[error("")]
    CleanProjectRoot { project_root: AbsPathBuf, err: io::Error },

    #[error("")]
    CreateProjectRoot { project_root: AbsPathBuf, err: io::Error },

    #[error("")]
    CreateDir { dir_path: AbsPathBuf, err: io::Error },

    #[error("")]
    CreateFile { file_path: AbsPathBuf, err: io::Error },

    #[error("")]
    WriteFile { file_path: AbsPathBuf, err: io::Error },
}

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct JumpToHostError;

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct RemoveProjectRootError {
    #[from]
    inner: io::Error,
}

impl JoinGuard {
    fn new(
        session_status: Shared<SessionStatus>,
        session_id: SessionId,
    ) -> Result<Self, UserBusyError<false>> {
        match session_status.with(|s| UserBusyError::try_from(s)).ok() {
            Some(err) => Err(err),
            None => {
                session_status.set(SessionStatus::Joining(session_id));
                Ok(Self { session_status })
            },
        }
    }

    fn set_in_session(&self, session: &Session) {
        self.session_status.set(SessionStatus::InSession(session.project()));
    }
}

impl ConnectToServer {
    async fn connect_to_server(
        self,
    ) -> Result<Authenticate, ConnectToServerError> {
        collab_server::Io::connect()
            .await
            .map(|io| Authenticate { io, guard: self.guard })
            .map_err(Into::into)
    }
}

impl Authenticate {
    async fn authenticate(
        self,
        auth_infos: AuthInfos,
    ) -> Result<JoinSession, collab_server::client::AuthError> {
        self.io.authenticate(auth_infos.clone()).await.map(|authenticated| {
            JoinSession { authenticated, auth_infos, guard: self.guard }
        })
    }
}

impl JoinSession {
    async fn join_session(
        self,
        session_id: SessionId,
    ) -> Result<ConfirmJoin, JoinSessionError> {
        self.authenticated
            .join(collab_server::client::JoinRequest::JoinExistingSession(
                session_id.into_inner(),
            ))
            .await
            .map(|joined| ConfirmJoin {
                local_peer: Peer::new(
                    joined.sender.peer_id(),
                    self.auth_infos.github_handle,
                ),
                joined,
                guard: self.guard,
            })
            .map_err(Into::into)
    }
}

impl ConfirmJoin {
    async fn confirm_join(self) -> Result<RequestProject, ConfirmJoinError> {
        Ok(RequestProject {
            joined: self.joined,
            local_peer: self.local_peer,
            guard: self.guard,
        })
    }
}

impl RequestProject {
    async fn request_project(
        mut self,
    ) -> Result<FindProjectRoot, RequestProjectError> {
        let request_from = self
            .joined
            .peers
            .as_slice()
            .first()
            .expect("can't be empty")
            .clone();

        self.joined
            .sender
            .send(Message::ProjectRequest(ProjectRequest { request_from }))
            .await
            .map_err(RequestProjectError::SendRequest)?;

        let mut buffered = Vec::new();

        let project_response = loop {
            let res = self.joined.receiver.next().await.expect("never ends");
            let message = res.map_err(RequestProjectError::ReadResponse)?;
            match message {
                Message::ProjectResponse(response) => break response,
                other => buffered.push(other),
            };
        };

        Ok(FindProjectRoot {
            buffered,
            joined: self.joined,
            guard: self.guard,
            local_peer: self.local_peer,
            project_response,
        })
    }
}

impl FindProjectRoot {
    async fn find_project_root(
        self,
    ) -> Result<FlushProject, FindProjectRootError> {
        let project_root = "/home/noib3/.local/share/nvim/collab/nomad.nvim"
            .parse()
            .expect("it's valid");

        Ok(FlushProject {
            buffered: self.buffered,
            joined: self.joined,
            local_peer: self.local_peer,
            guard: self.guard,
            project_response: self.project_response,
            project_root,
        })
    }
}

impl FlushProject {
    async fn flush_project(
        self,
        neovim_ctx: NeovimCtx<'_>,
    ) -> Result<JumpToHost, FlushProjectError> {
        if async_fs::metadata(&self.project_root).await.is_ok() {
            // Clean project root.
            async_fs::remove_dir_all(&self.project_root).await.map_err(
                |err| FlushProjectError::CleanProjectRoot {
                    project_root: self.project_root.clone(),
                    err,
                },
            )?;
        }

        // Create all missing directories to project root.
        async_fs::create_dir_all(&self.project_root).await.map_err(|err| {
            FlushProjectError::CreateProjectRoot {
                project_root: self.project_root.clone(),
                err,
            }
        })?;

        let (err_tx, err_rx) = flume::unbounded();
        let encoded_replica = self.project_response.replica;
        let replica = Replica::decode(self.local_peer.id(), encoded_replica);
        let file_contents = self.project_response.file_contents;
        let root_id = replica.root().id();
        let tree = Rc::new(ProjectTree { replica, file_contents });
        recurse(
            Rc::clone(&tree),
            root_id,
            self.project_root.clone(),
            ErrTx { has_errored: Shared::new(false), inner: err_tx },
            neovim_ctx,
        );

        let ProjectTree { replica, .. } = match err_rx.recv_async().await {
            Ok(err) => return Err(err),
            Err(_all_senders_dropped_err) => {
                // All the senders have been dropped, so all the other
                // instances of the tree must have been dropped as well.
                Rc::into_inner(tree).expect("recursion has ended")
            },
        };

        Ok(JumpToHost {
            buffered: self.buffered,
            joined: self.joined,
            local_peer: self.local_peer,
            project_root: self.project_root,
            remote_peers: self.project_response.peers,
            replica,
            guard: self.guard,
        })
    }
}

impl JumpToHost {
    fn jump_to_host(self) -> RunSession {
        RunSession {
            buffered: self.buffered,
            joined: self.joined,
            local_peer: self.local_peer,
            project_root: self.project_root,
            remote_peers: self.remote_peers,
            replica: self.replica,
            guard: self.guard,
        }
    }
}

impl RunSession {
    async fn run_session(
        self,
        neovim_ctx: NeovimCtx<'static>,
    ) -> (RemoveProjectRoot, Option<RunSessionError<io::Error, io::Error>>)
    {
        let collab_server::client::Joined {
            sender: tx,
            receiver: rx,
            session_id,
            peers: _,
        } = self.joined;

        let session = Session::new(NewSessionArgs {
            is_host: false,
            local_peer: self.local_peer,
            remote_peers: self.remote_peers,
            project_root: self.project_root.clone(),
            replica: self.replica,
            session_id,
            neovim_ctx,
        });

        self.guard.set_in_session(&session);

        let rx = stream::iter(self.buffered.into_iter().map(Ok)).chain(rx);
        let maybe_err = session.run(tx, rx).await.err();
        (RemoveProjectRoot { project_root: self.project_root }, maybe_err)
    }
}

impl RemoveProjectRoot {
    async fn remove_project_root(self) -> Result<(), RemoveProjectRootError> {
        async_fs::remove_dir(self.project_root).await.map_err(Into::into)
    }
}

struct ProjectTree {
    replica: Replica,
    file_contents: FileContents,
}

#[derive(Clone)]
struct ErrTx {
    has_errored: Shared<bool>,
    inner: flume::Sender<FlushProjectError>,
}

impl ErrTx {
    fn has_errored(&self) -> bool {
        self.has_errored.get()
    }

    fn send(self, err: FlushProjectError) {
        self.has_errored.set(true);
        self.inner.send(err).expect("receiver hasn't been dropped");
    }
}

#[allow(clippy::too_many_arguments)]
fn recurse(
    tree: Rc<ProjectTree>,
    parent_id: DirectoryId,
    parent_path: AbsPathBuf,
    err_tx: ErrTx,
    ctx: NeovimCtx<'_>,
) {
    if err_tx.has_errored() {
        return;
    }

    ctx.spawn(|ctx| async move {
        let replica = &tree.replica;
        let file_contents = &tree.file_contents;
        let parent = replica.directory(parent_id).expect("ID is valid");

        let create_directories = parent.child_directories().map(|dir| {
            let mut dir_path = parent_path.clone();
            dir_path.push(dir.name().expect("can't be root"));
            let err_tx = err_tx.clone();
            create_directory(&tree, dir, dir_path, err_tx, ctx.reborrow())
        });

        let create_files = parent.child_files().map(|file| {
            let mut file_path = parent_path.clone();
            file_path.push(file.name());
            let contents = file_contents.get(file.id()).expect("ID is valid");
            create_file(contents, file_path)
        });

        let mut create_children = create_directories
            .map(future::Either::Left)
            .chain(create_files.map(future::Either::Right))
            .collect::<stream::FuturesUnordered<_>>();

        while let Some(res) = create_children.next().await {
            if let Err(err) = res {
                err_tx.send(err);
                return;
            }
        }
    })
    .detach();
}

async fn create_file(
    file_contents: &str,
    file_path: AbsPathBuf,
) -> Result<(), FlushProjectError> {
    let mut file = match async_fs::File::create(&file_path).await {
        Ok(file) => file,
        Err(err) => {
            return Err(FlushProjectError::CreateFile { file_path, err });
        },
    };
    if let Err(err) = file.write_all(file_contents.as_bytes()).await {
        return Err(FlushProjectError::WriteFile { file_path, err });
    }
    match file.flush().await {
        Ok(()) => Ok(()),
        Err(err) => Err(FlushProjectError::WriteFile { file_path, err }),
    }
}

#[allow(clippy::too_many_arguments)]
async fn create_directory(
    tree: &Rc<ProjectTree>,
    dir: DirectoryRef<'_>,
    dir_path: AbsPathBuf,
    err_tx: ErrTx,
    ctx: NeovimCtx<'_>,
) -> Result<(), FlushProjectError> {
    match async_fs::create_dir(&dir_path).await {
        Ok(()) => {
            recurse(Rc::clone(tree), dir.id(), dir_path, err_tx, ctx);
            Ok(())
        },

        Err(err) => Err(FlushProjectError::CreateDir {
            dir_path: dir_path.to_owned(),
            err,
        }),
    }
}

trait JoinStep {
    const MESSAGE: &'static str;
}

fn spin<T: JoinStep>() -> impl Stream<Item = DiagnosticMessage> {
    stream::empty()
}

impl JoinStep for ConnectToServer {
    const MESSAGE: &'static str = "Connecting to server";
}

impl JoinStep for Authenticate {
    const MESSAGE: &'static str = "Authenticating";
}

impl JoinStep for JoinSession {
    const MESSAGE: &'static str = "Joining session";
}

impl JoinStep for ConfirmJoin {
    const MESSAGE: &'static str = "Confirming join";
}

impl JoinStep for RequestProject {
    const MESSAGE: &'static str = "Requesting project";
}

impl JoinStep for FindProjectRoot {
    const MESSAGE: &'static str = "Finding project root";
}

impl JoinStep for FlushProject {
    const MESSAGE: &'static str = "Flushing project";
}

impl Drop for JoinGuard {
    fn drop(&mut self) {
        self.session_status.set(SessionStatus::NotInSession);
    }
}

impl From<JoinError> for DiagnosticMessage {
    fn from(err: JoinError) -> Self {
        let mut message = Self::new();
        message.push_str(err.to_string());
        message
    }
}
