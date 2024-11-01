use collab_server::message::{GitHubHandle, Message};
use collab_server::AuthInfos;
use futures_util::StreamExt;
use nomad::ctx::NeovimCtx;
use nomad::diagnostics::DiagnosticMessage;
use nomad::{action_name, ActionName, AsyncAction, Shared};

use super::UserBusyError;
use crate::session::Session;
use crate::session_status::SessionStatus;
use crate::Collab;

#[derive(Clone)]
pub(crate) struct Start {
    session_status: Shared<SessionStatus>,
}

impl Start {
    pub(crate) fn new(session_status: Shared<SessionStatus>) -> Self {
        Self { session_status }
    }
}

impl AsyncAction for Start {
    const NAME: ActionName = action_name!("start");
    type Args = ();
    type Docs = ();
    type Module = Collab;

    async fn execute(
        &mut self,
        _: Self::Args,
        ctx: NeovimCtx<'_>,
    ) -> Result<(), StartError> {
        let auth_infos = AuthInfos {
            github_handle: "noib3"
                .parse::<GitHubHandle>()
                .expect("it's valid"),
        };

        #[rustfmt::skip]
        Starter::new(self.session_status.clone(), ctx.to_static())?
            .find_project_root().await?
            .confirm_start().await?
            .read_project().await?
            .connect_to_server().await?
            .authenticate(auth_infos).await?
            .start_session().await?
            .run_session().await?;

        Ok(())
    }

    fn docs(&self) -> Self::Docs {}
}

struct Starter<State> {
    session_status: Shared<SessionStatus>,
    state: State,
}

struct FindProjectRoot {
    ctx: NeovimCtx<'static>,
}

struct ConfirmStart;
struct ReadProject;
struct ConnectToServer;
struct Authenticate;
struct StartSession;
struct RunSession;

#[derive(Debug, thiserror::Error)]
pub(crate) enum StartError {
    #[error(transparent)]
    ConfirmStart(#[from] ConfirmStartError),

    #[error(transparent)]
    ReadProject(#[from] ReadProjectError),

    #[error(transparent)]
    ConnectToServer(#[from] ConnectToServerError),

    #[error(transparent)]
    Authenticate(#[from] AuthenticateError),

    #[error(transparent)]
    StartSession(#[from] StartSessionError),

    #[error(transparent)]
    RunSession(#[from] RunSessionError),

    #[error(transparent)]
    FindProjectRoot(#[from] FindProjectRootError),

    #[error(transparent)]
    UserBusy(#[from] UserBusyError<true>),
}

#[derive(Debug, thiserror::Error)]
#[error("failed to find project root")]
pub(crate) struct FindProjectRootError;

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct ConfirmStartError;

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct ReadProjectError;

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct ConnectToServerError;

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct AuthenticateError;

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct StartSessionError;

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct RunSessionError;

impl Starter<FindProjectRoot> {
    fn new(
        session_status: Shared<SessionStatus>,
        ctx: NeovimCtx<'static>,
    ) -> Result<Self, UserBusyError<true>> {
        match session_status.with(|s| UserBusyError::try_from(s)).ok() {
            Some(err) => Err(err),
            None => {
                session_status.set(SessionStatus::Starting);
                Ok(Self { session_status, state: FindProjectRoot { ctx } })
            },
        }
    }

    async fn find_project_root(
        self,
    ) -> Result<Starter<ConfirmStart>, FindProjectRootError> {
        todo!();
    }
}

impl Starter<ConfirmStart> {
    async fn confirm_start(
        self,
    ) -> Result<Starter<ReadProject>, ConfirmStartError> {
        todo!();
    }
}

impl Starter<ReadProject> {
    async fn read_project(
        self,
    ) -> Result<Starter<ConnectToServer>, ReadProjectError> {
        todo!();
    }
}

impl Starter<ConnectToServer> {
    async fn connect_to_server(
        self,
    ) -> Result<Starter<Authenticate>, ConnectToServerError> {
        todo!();
    }
}

impl Starter<Authenticate> {
    async fn authenticate(
        self,
        _auth_infos: AuthInfos,
    ) -> Result<Starter<StartSession>, AuthenticateError> {
        todo!();
    }
}

impl Starter<StartSession> {
    async fn start_session(
        self,
    ) -> Result<Starter<RunSession>, StartSessionError> {
        todo!();
    }
}

impl Starter<RunSession> {
    async fn run_session(self) -> Result<(), RunSessionError> {
        todo!();
    }
}

impl<State> From<State> for Starter<State> {
    fn from(_state: State) -> Self {
        todo!();
    }
}

impl From<StartError> for DiagnosticMessage {
    fn from(_err: StartError) -> Self {
        todo!();
    }
}

// let mut session = Session::start().await;
// self.session_status.set(SessionStatus::InSession(session.project()));
// ctx.spawn(async move {
//     let (tx, rx) = flume::unbounded::<Message>();
//     let tx = tx.into_sink::<'static>();
//     let rx = rx
//         .into_stream::<'static>()
//         .map(Ok::<_, core::convert::Infallible>);
//     let _err = session.run(tx, rx).await;
// });
