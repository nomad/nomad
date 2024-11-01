use std::io;

use collab_server::message::{
    GitHubHandle,
    Message,
    Peer,
    Peers,
    ProjectRequest,
};
use collab_server::AuthInfos;
use e31e::fs::AbsPathBuf;
use e31e::Replica;
use futures_util::{SinkExt, StreamExt};
use nomad::ctx::NeovimCtx;
use nomad::diagnostics::DiagnosticMessage;
use nomad::{action_name, ActionName, AsyncAction, Shared};

use super::UserBusyError;
use crate::session::{NewSessionArgs, Session};
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

        #[rustfmt::skip]
        Joiner::new(self.session_status.clone(), session_id, ctx.to_static())?
            .connect_to_server().await?
            .authenticate(auth_infos).await?
            .join_session(session_id.into_inner()).await?
            .confirm_join().await?
            .request_project().await?
            .find_project_root().await?
            .flush_project().await?
            .jump_to_host()
            .run_session().await?
            .remove_project_root().await;

        Ok(())
    }

    fn docs(&self) {}
}

struct Joiner {
    ctx: NeovimCtx<'static>,
    session_id: SessionId,
    session_status: Shared<SessionStatus>,
}

struct Authenticate {
    io: collab_server::Io,
    joiner: Joiner,
}

struct JoinSession {
    authenticated: collab_server::client::Authenticated,
    auth_infos: AuthInfos,
    joiner: Joiner,
}

struct ConfirmJoin {
    joined: collab_server::client::Joined,
    local_peer: Peer,
    joiner: Joiner,
}

struct RequestProject {
    joined: collab_server::client::Joined,
    local_peer: Peer,
    joiner: Joiner,
}

struct FindProjectRoot {
    buffered: Vec<Message>,
    local_peer: Peer,
    joined: collab_server::client::Joined,
    project: collab_server::message::Project,
    joiner: Joiner,
}

struct FlushProject {
    buffered: Vec<Message>,
    local_peer: Peer,
    joined: collab_server::client::Joined,
    project: collab_server::message::Project,
    project_root: AbsPathBuf,
    joiner: Joiner,
}

struct JumpToHost {
    buffered: Vec<Message>,
    joined: collab_server::client::Joined,
    local_peer: Peer,
    project: collab_server::message::Project,
    project_root: AbsPathBuf,
    joiner: Joiner,
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
    joiner: Joiner,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum JoinError {
    #[error(transparent)]
    ConnectToServer(#[from] ConnectToServerError),

    #[error(transparent)]
    Authenticate(#[from] AuthenticateError),

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
    RunSession(#[from] RunSessionError),

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
pub(crate) struct AuthenticateError {
    inner: (),
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
#[error("")]
pub(crate) struct FlushProjectError;

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct JumpToHostError;

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct RunSessionError;

impl Joiner {
    fn new(
        session_status: Shared<SessionStatus>,
        session_id: SessionId,
        ctx: NeovimCtx<'static>,
    ) -> Result<Self, UserBusyError<false>> {
        match session_status.with(|s| UserBusyError::try_from(s)).ok() {
            Some(err) => Err(err),
            None => {
                session_status.set(SessionStatus::Joining(session_id));
                Ok(Self { ctx, session_id, session_status })
            },
        }
    }

    async fn connect_to_server(
        self,
    ) -> Result<Authenticate, ConnectToServerError> {
        collab_server::Io::connect()
            .await
            .map(|io| Authenticate { io, joiner: self })
            .map_err(Into::into)
    }
}

impl Authenticate {
    async fn authenticate(
        self,
        auth_infos: AuthInfos,
    ) -> Result<JoinSession, AuthenticateError> {
        self.io
            .authenticate(auth_infos.clone())
            .await
            .map(|authenticated| JoinSession {
                authenticated,
                auth_infos,
                joiner: self.joiner,
            })
            .map_err(|_err| todo!())
    }
}

impl JoinSession {
    async fn join_session(
        self,
        session_id: collab_server::SessionId,
    ) -> Result<ConfirmJoin, JoinSessionError> {
        self.authenticated
            .join(collab_server::client::JoinRequest::JoinExistingSession(
                session_id,
            ))
            .await
            .map(|joined| ConfirmJoin {
                local_peer: Peer::new(
                    joined.sender.peer_id(),
                    self.auth_infos.github_handle,
                ),
                joined,
                joiner: self.joiner,
            })
            .map_err(Into::into)
    }
}

impl ConfirmJoin {
    async fn confirm_join(self) -> Result<RequestProject, ConfirmJoinError> {
        todo!();
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

        let project = loop {
            let res = self.joined.receiver.next().await.expect("never ends");
            let message = res.map_err(RequestProjectError::ReadResponse)?;
            match message {
                Message::ProjectResponse(response) => break response.project,
                other => buffered.push(other),
            };
        };

        Ok(FindProjectRoot {
            buffered,
            joined: self.joined,
            joiner: self.joiner,
            local_peer: self.local_peer,
            project,
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
            joiner: self.joiner,
            project: self.project,
            project_root,
        })
    }
}

impl FlushProject {
    async fn flush_project(self) -> Result<JumpToHost, FlushProjectError> {
        todo!();
    }
}

impl JumpToHost {
    fn jump_to_host(self) -> RunSession {
        todo!();
    }
}

impl RunSession {
    async fn run_session(self) -> Result<RemoveProjectRoot, RunSessionError> {
        let collab_server::client::Joined {
            sender,
            receiver,
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
            neovim_ctx: self.joiner.ctx,
        });

        let status = SessionStatus::InSession(session.project());
        self.joiner.session_status.set(status);
        session.run(sender, receiver).await.map_err(|_err| todo!())?;

        Ok(RemoveProjectRoot { project_root: self.project_root })
    }
}

impl RemoveProjectRoot {
    async fn remove_project_root(self) {
        let _ = async_fs::remove_dir(self.project_root).await;
    }
}

impl From<JoinError> for DiagnosticMessage {
    fn from(_err: JoinError) -> Self {
        todo!();
    }
}
