use collab_server::message::{GitHubHandle, Message};
use collab_server::AuthInfos;
use futures_util::StreamExt;
use nomad::ctx::NeovimCtx;
use nomad::diagnostics::DiagnosticMessage;
use nomad::{action_name, ActionName, AsyncAction, Shared};

use super::UserBusyError;
use crate::session::Session;
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
            .join_session(session_id).await?
            .confirm_join().await?
            .request_project().await?
            .find_project_root().await?
            .flush_project().await?
            .jump_to_host().await?
            .run_session().await?;

        Ok(())
    }

    fn docs(&self) {}
}

struct Joiner<State> {
    session_status: Shared<SessionStatus>,
    state: State,
}

struct ConnectToServer {
    ctx: NeovimCtx<'static>,
}

struct Authenticate;
struct JoinSession;
struct ConfirmJoin;
struct RequestProject;
struct FindProjectRoot;
struct FlushProject;
struct JumpToHost;
struct RunSession;

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
pub(crate) struct ConnectToServerError;

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct AuthenticateError;

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct JoinSessionError;

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct ConfirmJoinError;

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct RequestProjectError;

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

impl Joiner<ConnectToServer> {
    fn new(
        session_status: Shared<SessionStatus>,
        session_id: SessionId,
        ctx: NeovimCtx<'static>,
    ) -> Result<Self, UserBusyError<false>> {
        match session_status.with(|s| UserBusyError::try_from(s)).ok() {
            Some(err) => Err(err),
            None => {
                session_status.set(SessionStatus::Joining(session_id));
                Ok(Self { session_status, state: ConnectToServer { ctx } })
            },
        }
    }

    async fn connect_to_server(
        self,
    ) -> Result<Joiner<Authenticate>, JoinError> {
        todo!();
    }
}

impl Joiner<Authenticate> {
    async fn authenticate(
        self,
        _auth_infos: AuthInfos,
    ) -> Result<Joiner<JoinSession>, JoinError> {
        todo!();
    }
}

impl Joiner<JoinSession> {
    async fn join_session(
        self,
        _session_id: SessionId,
    ) -> Result<Joiner<ConfirmJoin>, JoinError> {
        todo!();
    }
}

impl Joiner<ConfirmJoin> {
    async fn confirm_join(self) -> Result<Joiner<RequestProject>, JoinError> {
        todo!();
    }
}

impl Joiner<RequestProject> {
    async fn request_project(
        self,
    ) -> Result<Joiner<FindProjectRoot>, JoinError> {
        todo!();
    }
}

impl Joiner<FindProjectRoot> {
    async fn find_project_root(
        self,
    ) -> Result<Joiner<FlushProject>, JoinError> {
        todo!();
    }
}

impl Joiner<FlushProject> {
    async fn flush_project(self) -> Result<Joiner<JumpToHost>, JoinError> {
        todo!();
    }
}

impl Joiner<JumpToHost> {
    async fn jump_to_host(self) -> Result<Joiner<RunSession>, JoinError> {
        todo!();
    }
}

impl Joiner<RunSession> {
    async fn run_session(self) -> Result<(), JoinError> {
        todo!();
    }
}

impl<State> From<State> for Joiner<State> {
    fn from(_state: State) -> Self {
        todo!();
    }
}

impl From<JoinError> for DiagnosticMessage {
    fn from(_err: JoinError) -> Self {
        todo!();
    }
}

// let mut session = Session::join().await;
// self.session_status.set(SessionStatus::InSession(session.project()));
// ctx.spawn(async move {
//     let (tx, rx) = flume::unbounded::<Message>();
//     let tx = tx.into_sink::<'static>();
//     let rx = rx
//         .into_stream::<'static>()
//         .map(Ok::<_, core::convert::Infallible>);
//     let _err = session.run(tx, rx).await;
// });
