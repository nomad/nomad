use core::time::Duration;
use std::io;
use std::rc::Rc;

use async_net::TcpStream;
use collab_server::client::{ClientRxError, KnockError, Knocker, Welcome};
use collab_server::configs::nomad::{
    NomadAuthenticateInfos,
    NomadAuthenticator,
    NomadConfig,
};
use collab_server::message::{
    FileContents,
    GitHubHandle,
    Message,
    Peer,
    Peers,
    ProjectRequest,
    ProjectResponse,
};
use collab_server::SessionIntent;
use eerie::fs::AbsPathBuf;
use eerie::{DirectoryId, DirectoryRef, Replica};
use futures_util::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use futures_util::{future, stream, SinkExt, Stream, StreamExt};
use nvimx::ctx::NeovimCtx;
use nvimx::diagnostics::DiagnosticMessage;
use nvimx::emit::{Emit, EmitExt, EmitMessage, Severity};
use nvimx::plugin::{action_name, ActionName, AsyncAction, ToCompletionFunc};
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
        let auth_infos = NomadAuthenticateInfos {
            github_handle: "noib3"
                .parse::<GitHubHandle>()
                .expect("it's valid"),
        };

        let guard = JoinGuard::new(self.session_status.clone(), session_id)?;

        #[rustfmt::skip]
        let step = ConnectToServer { guard }
            .connect_to_server().emitting(spin::<ConnectToServer>()).await?
            .knock(auth_infos, session_id).emitting(spin::<Knock>()).await?
            .confirm_join().await?
            .request_project().emitting(spin::<RequestProject>()).await?
            .find_project_root().emitting(spin::<FindProjectRoot>()).await?
            .flush_project(ctx.reborrow()).emitting(spin::<FlushProject>()).await?
            .jump_to_host();

        JoinedProject(&step).clear_after(Duration::from_secs(4)).emit();

        let (step, maybe_err) = step.run_session(ctx.to_static()).await;

        step.remove_project_root()
            .await
            .map_err(Into::into)
            .map_err(|err| maybe_err.map(Into::into).unwrap_or(err))
    }

    fn docs(&self) {}
}

impl ToCompletionFunc for Join {}

struct JoinGuard {
    session_status: Shared<SessionStatus>,
}

struct ConnectToServer {
    guard: JoinGuard,
}

struct Knock {
    io: TcpStream,
    guard: JoinGuard,
}

struct ConfirmJoin {
    local_peer: Peer,
    welcome: Welcome<ReadHalf<TcpStream>, WriteHalf<TcpStream>>,
    guard: JoinGuard,
}

struct RequestProject {
    local_peer: Peer,
    welcome: Welcome<ReadHalf<TcpStream>, WriteHalf<TcpStream>>,
    guard: JoinGuard,
}

struct FindProjectRoot {
    buffered: Vec<Message>,
    local_peer: Peer,
    welcome: Welcome<ReadHalf<TcpStream>, WriteHalf<TcpStream>>,
    project_response: Box<ProjectResponse>,
    guard: JoinGuard,
}

struct FlushProject {
    buffered: Vec<Message>,
    local_peer: Peer,
    welcome: Welcome<ReadHalf<TcpStream>, WriteHalf<TcpStream>>,
    project_response: Box<ProjectResponse>,
    project_root: AbsPathBuf,
    guard: JoinGuard,
}

struct JumpToHost {
    buffered: Vec<Message>,
    welcome: Welcome<ReadHalf<TcpStream>, WriteHalf<TcpStream>>,
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
    local_peer: Peer,
    project_root: AbsPathBuf,
    remote_peers: Peers,
    replica: Replica,
    welcome: Welcome<ReadHalf<TcpStream>, WriteHalf<TcpStream>>,
    guard: JoinGuard,
}

struct JoinedProject<'a>(&'a RunSession);

#[derive(Debug, thiserror::Error)]
pub(crate) enum JoinError {
    #[error(transparent)]
    ConnectToServer(#[from] ConnectToServerError),

    #[error(transparent)]
    Knock(#[from] KnockError<NomadAuthenticator>),

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
    RunSession(#[from] RunSessionError<io::Error, ClientRxError>),

    #[error(transparent)]
    RemoveProjectRoot(#[from] RemoveProjectRootError),

    #[error(transparent)]
    UserBusy(#[from] UserBusyError<false>),
}

#[derive(Debug, thiserror::Error)]
#[error("couldn't connect to server: {inner}")]
pub(crate) struct ConnectToServerError {
    #[from]
    inner: io::Error,
}

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct ConfirmJoinError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum RequestProjectError {
    #[error("couldn't send project request: {0}")]
    SendRequest(io::Error),

    #[error("couldn't read project response: {0}")]
    ReadResponse(ClientRxError),
}

#[derive(Debug, thiserror::Error)]
#[error("couldn't find project root")]
pub(crate) struct FindProjectRootError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum FlushProjectError {
    #[error("couldn't remove project root at `{project_root}`: {err}")]
    CleanProjectRoot { project_root: AbsPathBuf, err: io::Error },

    #[error("couldn't create project root at `{project_root}`: {err}")]
    CreateProjectRoot { project_root: AbsPathBuf, err: io::Error },

    #[error("couldn't create project directory at `{dir_path}`: {err}")]
    CreateDir { dir_path: AbsPathBuf, err: io::Error },

    #[error("couldn't create project file at `{file_path}`: {err}")]
    CreateFile { file_path: AbsPathBuf, err: io::Error },

    #[error("couldn't write to project file at `{file_path}`: {err}")]
    WriteFile { file_path: AbsPathBuf, err: io::Error },
}

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct JumpToHostError;

#[derive(Debug, thiserror::Error)]
#[error("couldn't remove project root: {inner:?}")]
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
    async fn connect_to_server(self) -> Result<Knock, ConnectToServerError> {
        todo!();
    }
}

impl Knock {
    async fn knock(
        self,
        auth_infos: NomadAuthenticateInfos,
        session_id: SessionId,
    ) -> Result<ConfirmJoin, KnockError<NomadAuthenticator>> {
        let (reader, writer) = self.io.split();
        let github_handle = auth_infos.github_handle.clone();
        let knock = collab_server::Knock {
            auth_infos,
            session_intent: SessionIntent::JoinExisting(
                session_id.into_inner(),
            ),
        };
        let welcome = Knocker::<_, _, NomadConfig>::new(reader, writer)
            .knock(knock)
            .await?;
        Ok(ConfirmJoin {
            local_peer: Peer::new(welcome.peer_id, github_handle),
            welcome,
            guard: self.guard,
        })
    }
}

impl ConfirmJoin {
    async fn confirm_join(self) -> Result<RequestProject, ConfirmJoinError> {
        Ok(RequestProject {
            welcome: self.welcome,
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
            .welcome
            .other_peers
            .as_slice()
            .first()
            .expect("can't be empty")
            .id();

        self.welcome
            .tx
            .send(Message::ProjectRequest(ProjectRequest {
                request_from,
                requested_by: self.local_peer.clone(),
            }))
            .await
            .map_err(RequestProjectError::SendRequest)?;

        let mut buffered = Vec::new();

        let project_response = loop {
            let res = self.welcome.rx.next().await.expect("never ends");
            let message = res.map_err(RequestProjectError::ReadResponse)?;
            match message {
                Message::ProjectResponse(response) => break response,
                other => buffered.push(other),
            };
        };

        Ok(FindProjectRoot {
            buffered,
            welcome: self.welcome,
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
            welcome: self.welcome,
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
        let replica = Replica::decode(self.welcome.peer_id, encoded_replica);
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
            welcome: self.welcome,
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
            welcome: self.welcome,
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
    ) -> (RemoveProjectRoot, Option<RunSessionError<io::Error, ClientRxError>>)
    {
        let Welcome { session_id, tx, rx, .. } = self.welcome;

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

struct SpinFrame {
    spinner: Spinner,
    step_message: &'static str,
}

#[derive(Copy, Clone)]
#[repr(u8)]
#[allow(dead_code)]
enum Spinner {
    Iter1 = 0,
    Iter2,
    Iter3,
}

fn spin<T: JoinStep>() -> impl Stream<Item = SpinFrame> {
    async_stream::stream! {
        let mut spinner = Spinner::Iter1;
        loop {
            yield SpinFrame::new(spinner, T::MESSAGE);
            spinner.advance();
            nvimx::executor::sleep(SpinFrame::DURATION).await;
        }
    }
}

impl SpinFrame {
    const DURATION: Duration = Duration::from_millis(80);

    fn new(spinner: Spinner, step_message: &'static str) -> Self {
        Self { spinner, step_message }
    }
}

impl Spinner {
    const NUM_FRAMES: u8 = 3;

    fn advance(&mut self) {
        // SAFETY: `Self` is `repr(u8)`.
        *self = unsafe {
            core::mem::transmute::<u8, Self>(
                *self as u8 + 1 % Self::NUM_FRAMES,
            )
        };
    }

    fn into_char(self) -> char {
        match self {
            Self::Iter1 => '🌍',
            Self::Iter2 => '🌎',
            Self::Iter3 => '🌏',
        }
    }
}

impl Emit for SpinFrame {
    const ADD_TO_MESSAGE_HISTORY: bool = false;

    type Action = Join;

    fn message(&self) -> EmitMessage {
        let mut msg = EmitMessage::new();
        msg.push(self.spinner.into_char())
            .push_str(" ")
            .push_str(self.step_message);
        msg
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }
}

impl Emit for JoinedProject<'_> {
    const ADD_TO_MESSAGE_HISTORY: bool = true;

    type Action = Join;

    fn message(&self) -> EmitMessage {
        let Self(run) = self;
        let mut msg = EmitMessage::new();
        msg.push_str("joined project at ").push_str(&run.project_root);
        msg
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }
}

impl JoinStep for ConnectToServer {
    const MESSAGE: &'static str = "Connecting to server";
}

impl JoinStep for Knock {
    const MESSAGE: &'static str = "Authenticating";
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
