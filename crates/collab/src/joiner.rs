use core::fmt;
use core::marker::PhantomData;
use std::io;

use collab_fs::AbsUtf8PathBuf;
use root_finder::markers::Git;
use root_finder::Finder;

use crate::{CollabEditor, Session, SessionId};

pub(crate) struct Joiner<Status> {
    status: Status,
}

impl Joiner<ConnectToServer> {
    pub(crate) fn new() -> Self {
        Self { status: ConnectToServer }
    }

    pub(crate) async fn connect_to_server(
        self,
    ) -> Result<Joiner<Authenticate>, ConnectToServerError> {
        todo!();
    }
}

impl Joiner<Authenticate> {
    pub(crate) async fn authenticate(
        self,
        _auth_infos: (),
    ) -> Result<Joiner<JoinSession>, AuthenticateError> {
        todo!();
    }
}

impl Joiner<JoinSession> {
    pub(crate) async fn join_session(
        self,
        _session_id: SessionId,
    ) -> Result<Joiner<ConfirmJoin>, JoinSessionError> {
        todo!();
    }
}

impl Joiner<ConfirmJoin> {
    pub(crate) async fn confirm_join<E: CollabEditor>(
        self,
        _editor: &E,
    ) -> Result<Joiner<AskForProject>, ConfirmJoinError> {
        todo!();
    }
}

impl Joiner<AskForProject> {
    pub(crate) async fn ask_for_project(
        self,
    ) -> Result<Joiner<CreateProjectTree>, AskForProjectError> {
        todo!();
    }
}

impl Joiner<CreateProjectTree> {
    pub(crate) async fn create_project_tree<E: CollabEditor>(
        self,
        _editor: &E,
    ) -> Result<Joiner<FocusBusiestFile>, CreateProjectTreeError> {
        todo!();
    }
}

impl Joiner<FocusBusiestFile> {
    pub(crate) fn focus_busiest_file<E: CollabEditor>(
        self,
        _editor: &E,
    ) -> Joiner<Done> {
        todo!();
    }
}

impl Joiner<Done> {
    pub(crate) fn into_session<E: CollabEditor>(
        self,
        _editor: E,
    ) -> Session<E> {
        todo!();
    }
}

pub(crate) struct Starter<Status> {
    status: Status,
}

impl<Status> From<Status> for Starter<Status> {
    fn from(status: Status) -> Self {
        Self { status }
    }
}

impl Starter<FindProjectRoot> {
    pub(crate) fn new() -> Self {
        Self { status: FindProjectRoot }
    }

    pub(crate) async fn find_project_root<E: CollabEditor>(
        self,
        editor: &E,
    ) -> Result<Starter<ConfirmStart>, FindProjectRootError> {
        let file_path = match editor.current_file() {
            Some(file_id) => editor.path(&file_id),
            None => return Err(FindProjectRootError::NotInFile),
        };

        match Finder::find_root(file_path.as_ref(), &Git, &editor.fs()).await {
            Ok(Some(candidate)) => Ok(ConfirmStart(candidate).into()),
            Ok(None) => Err(FindProjectRootError::CouldntFindRoot(
                file_path.into_owned(),
            )),
            Err(io_err) => {
                Err(FindProjectRootError::FailedLookingForRoot(io_err))
            },
        }
    }
}

impl Starter<ConfirmStart> {
    pub(crate) async fn confirm_start<E: CollabEditor>(
        self,
        editor: &E,
    ) -> Result<Starter<ConnectToServer>, ConfirmStartError> {
        struct StartSessionPrompt<'a>(&'a collab_fs::AbsUtf8PathBuf);
        impl fmt::Display for StartSessionPrompt<'_> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    f,
                    "found root of project at '{}'. Start session?",
                    self.0
                )
            }
        }

        let Self { status: ConfirmStart(root_candidate) } = self;
        match editor.ask_user(StartSessionPrompt(&root_candidate)).await {
            Some(true) => Ok(ConnectToServer.into()),
            Some(false) => Err(ConfirmStartError::UserCancelled),
            None => Err(ConfirmStartError::UserDismissed),
        }
    }
}

impl Starter<ConnectToServer> {
    pub(crate) async fn connect_to_server(
        self,
    ) -> Result<Starter<Authenticate>, ConnectToServerError> {
        match nomad_server::Io::connect().await {
            Ok(io) => Ok(Authenticate { io }.into()),
            Err(io_err) => Err(ConnectToServerError { inner: io_err }),
        }
    }
}

impl Starter<Authenticate> {
    pub(crate) async fn authenticate(
        self,
        auth_infos: (),
    ) -> Result<Starter<StartSession>, AuthenticateError> {
        let Self { status: Authenticate { io } } = self;
        match io.authenticate(auth_infos).await {
            Ok(authenticated) => Ok(StartSession { authenticated }.into()),
            Err(auth_error) => Err(AuthenticateError { inner: auth_error }),
        }
    }
}

impl Starter<StartSession> {
    pub(crate) async fn start_session(
        self,
    ) -> Result<Starter<ReadProjectTree>, StartSessionError> {
        let Self { status: StartSession { authenticated } } = self;
        let request = collab_server::JoinRequest::StartNewSession;
        match authenticated.join(request).await {
            Ok(joined) => Ok(ReadProjectTree { joined }.into()),
            Err(err) => Err(StartSessionError { inner: err }),
        }
    }
}

impl Starter<ReadProjectTree> {
    pub(crate) async fn read_project_tree<Fs: collab_fs::Fs>(
        self,
        mut fs: Fs,
    ) -> Result<Starter<Done>, ReadProjectTreeError> {
        let Self { status: ReadProjectTree { joined } } = self;
        let project_root = AbsUtf8PathBuf::root();

        let nomad_server::client::Joined {
            sender,
            receiver,
            join_response,
            peers,
        } = joined;

        let collab_server::JoinResponse { session_id, client_id, server_id } =
            join_response;

        let peer_id = collab_project::PeerId::new(
            joined.join_response.client_id.into_u64(),
        );

        fs.set_root(project_root.clone());

        let project = collab_project::Project::from_fs(peer_id, &fs)
            .await
            .map_err(|err| ReadProjectTreeError { inner: err })?;

        let inner_session = InnerSession::new(
            // editor,
            session_id,
            peers,
            project,
            project_root,
        );

        Session::new(inner_session, sender, receiver, server_id)
    }
}

impl Starter<Done> {
    pub(crate) fn into_session<E: CollabEditor>(
        self,
        _editor: E,
    ) -> Session<E> {
        todo!();
    }
}

/// TODO: docs.
struct ConnectToServer;

/// TODO: docs.
struct Authenticate {
    io: nomad_server::Io,
}

/// TODO: docs.
struct JoinSession;

/// TODO: docs.
struct FindProjectRoot;

/// TODO: docs.
struct ConfirmJoin;

/// TODO: docs.
struct AskForProject;

/// TODO: docs.
struct CreateProjectTree;

/// TODO: docs.
struct FocusBusiestFile;

/// TODO: docs.
struct ConfirmStart(collab_fs::AbsUtf8PathBuf);

/// TODO: docs.
struct StartSession {
    authenticated: nomad_server::client::Authenticated,
}

/// TODO: docs.
struct ReadProjectTree {
    joined: nomad_server::client::Joined,
}

/// TODO: docs.
struct Done;

/// TODO: docs.
struct JoinExistingSession;

/// TODO: docs.
struct StartNewSession;

pub(crate) enum JoinError {
    ConnectToServer(ConnectToServerError),
}

pub(crate) enum StartError {
    ConnectToServer(ConnectToServerError),
}

/// TODO: docs.
pub(crate) struct ConnectToServerError {
    inner: io::Error,
}

/// TODO: docs.
pub(crate) struct AuthenticateError {
    inner: collab_server::client::AuthError<nomad_server::Auth>,
}

/// TODO: docs.
pub(crate) struct JoinSessionError;

/// Error returned when the user cancels the join session process.
pub(crate) struct ConfirmJoinError;

/// TODO: docs.
pub(crate) struct AskForProjectError;

/// TODO: docs.
pub(crate) struct CreateProjectTreeError;

/// TODO: docs.
pub(crate) enum FindProjectRootError {
    NotInFile,
    CouldntFindRoot(AbsUtf8PathBuf),
    FailedLookingForRoot(io::Error),
}

/// Error returned when the user cancels the start session process.
pub(crate) enum ConfirmStartError {
    UserCancelled,
    UserDismissed,
}

/// TODO: docs.
pub(crate) struct StartSessionError {
    inner: nomad_server::client::JoinError,
}

/// TODO: docs.
pub(crate) struct ReadProjectTreeError {
    inner: io::Error,
}

impl From<ConnectToServerError> for JoinError {
    fn from(err: ConnectToServerError) -> Self {
        JoinError::ConnectToServer(err)
    }
}

impl From<AuthenticateError> for JoinError {
    fn from(err: AuthenticateError) -> Self {
        todo!();
    }
}

impl From<JoinSessionError> for JoinError {
    fn from(err: JoinSessionError) -> Self {
        todo!();
    }
}

impl From<ConfirmJoinError> for JoinError {
    fn from(err: ConfirmJoinError) -> Self {
        todo!();
    }
}

impl From<AskForProjectError> for JoinError {
    fn from(err: AskForProjectError) -> Self {
        todo!();
    }
}

impl From<CreateProjectTreeError> for JoinError {
    fn from(err: CreateProjectTreeError) -> Self {
        todo!();
    }
}

impl From<FindProjectRootError> for StartError {
    fn from(err: FindProjectRootError) -> Self {
        todo!();
    }
}

impl From<ConfirmStartError> for StartError {
    fn from(err: ConfirmStartError) -> Self {
        todo!();
    }
}

impl From<ConnectToServerError> for StartError {
    fn from(err: ConnectToServerError) -> Self {
        StartError::ConnectToServer(err)
    }
}

impl From<AuthenticateError> for StartError {
    fn from(err: AuthenticateError) -> Self {
        todo!();
    }
}

impl From<StartSessionError> for StartError {
    fn from(err: StartSessionError) -> Self {
        todo!();
    }
}

impl From<ReadProjectTreeError> for StartError {
    fn from(err: ReadProjectTreeError) -> Self {
        todo!();
    }
}

// let Some(file_id) = editor.current_file() else {
//     return Err(StartSessionError::NotInFile);
// };
//
// let file_path = editor.path(&file_id);
//
// let Some(root_candidate) =
//     Finder::find_root(file_path.as_ref(), &Git, &editor.fs()).await?
// else {
//     return Err(StartSessionError::CouldntFindRoot(
//         file_path.into_owned(),
//     ));
// };
//
// let project_root =
//     match editor.ask_user(ConfirmStart(&root_candidate)).await {
//         Some(true) => root_candidate,
//         Some(false) => return Err(StartSessionError::UserCancelled),
//         None => todo!(),
//     };
//
// let joined = Io::connect()
//     .await?
//     .authenticate(())
//     .await?
//     .join(JoinRequest::StartNewSession)
//     .await?;
//
// let peer_id = PeerId::new(joined.join_response.client_id.into_u64());
//
// let project = Project::from_fs(peer_id, &editor.fs()).await?;
//
// todo!();

// Ok(Self::new(config, ctx, joined, project, project_root))
