use std::ffi::OsString;
use std::io;

use collab_server::message::{GitHubHandle, Peer, Peers};
use collab_server::AuthInfos;
use e31e::fs::{AbsPathBuf, FsNodeName};
use e31e::{Replica, ReplicaBuilder};
use futures_util::StreamExt;
use nvimx::ctx::{BufferCtx, NeovimCtx};
use nvimx::diagnostics::DiagnosticMessage;
use nvimx::fs::os_fs::OsFs;
use nvimx::plugin::{action_name, ActionName, AsyncAction};
use nvimx::Shared;
use root_finder::markers;

use super::UserBusyError;
use crate::session::{NewSessionArgs, RunSessionError, Session};
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
            .connect_to_server().await?
            .authenticate(auth_infos).await?
            .start_session().await?
            .read_replica().await?
            .run_session().await?;

        Ok(())
    }

    fn docs(&self) -> Self::Docs {}
}

struct Starter {
    ctx: NeovimCtx<'static>,
    session_status: Shared<SessionStatus>,
}

struct Authenticate {
    io: collab_server::Io,
    project_root: AbsPathBuf,
    starter: Starter,
}

struct ConfirmStart {
    project_root: AbsPathBuf,
    starter: Starter,
}

struct ConnectToServer {
    project_root: AbsPathBuf,
    starter: Starter,
}

struct ReadReplica {
    joined: collab_server::client::Joined,
    local_peer: Peer,
    project_root: AbsPathBuf,
    starter: Starter,
}

struct RunSession {
    joined: collab_server::client::Joined,
    local_peer: Peer,
    project_root: AbsPathBuf,
    replica: Replica,
    starter: Starter,
}

struct StartSession {
    authenticated: collab_server::client::Authenticated,
    auth_infos: AuthInfos,
    project_root: AbsPathBuf,
    starter: Starter,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum StartError {
    #[error(transparent)]
    Authenticate(#[from] collab_server::client::AuthError),

    #[error(transparent)]
    ConfirmStart(#[from] ConfirmStartError),

    #[error(transparent)]
    ConnectToServer(#[from] ConnectToServerError),

    #[error(transparent)]
    FindProjectRoot(#[from] FindProjectRootError),

    #[error(transparent)]
    ReadReplica(#[from] ReadReplicaError),

    #[error(transparent)]
    RunSession(#[from] RunSessionError<io::Error, io::Error>),

    #[error(transparent)]
    StartSession(#[from] StartSessionError),

    #[error(transparent)]
    UserBusy(#[from] UserBusyError<true>),
}

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct ConfirmStartError;

#[derive(Debug, thiserror::Error)]
#[error("couldn't connect to the server: {inner}")]
pub(crate) struct ConnectToServerError {
    #[from]
    inner: io::Error,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum FindProjectRootError {
    #[error(transparent)]
    FindRoot(#[from] root_finder::FindRootError<OsFs>),

    #[error("current buffer is not a file")]
    NotInFile,

    #[error("couldn't find the root of the project containing `{file_path}`")]
    UnknownRoot { file_path: AbsPathBuf },
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ReadReplicaError {
    /// The directory at the given path couldn't be read.
    #[error("")]
    CouldntReadDir { dir_path: AbsPathBuf, err: io::Error },

    /// The metadata of the node at the given path couldn't be read.
    #[error("")]
    CouldntReadMetadata { fs_node_path: AbsPathBuf, err: io::Error },

    /// The type of the node at the given path couldn't be read.
    #[error("")]
    CouldntReadType { fs_node_path: AbsPathBuf, err: io::Error },

    /// A node under the directory at the given path has a non-UTF-8 name.
    #[error("")]
    NodeNameNotUtf8 { parent_path: AbsPathBuf, fs_node_name: OsString },

    /// The given path was read twice.
    #[error("")]
    ReadDuplicate(AbsPathBuf),
}

#[derive(Debug, thiserror::Error)]
#[error("")]
pub(crate) struct StartSessionError {
    #[from]
    inner: collab_server::client::JoinError,
}

impl Starter {
    fn new(
        session_status: Shared<SessionStatus>,
        ctx: NeovimCtx<'static>,
    ) -> Result<Self, UserBusyError<true>> {
        match session_status.with(|s| UserBusyError::try_from(s)).ok() {
            Some(err) => Err(err),
            None => {
                session_status.set(SessionStatus::Starting);
                Ok(Self { ctx, session_status })
            },
        }
    }

    async fn find_project_root(
        self,
    ) -> Result<ConfirmStart, FindProjectRootError> {
        let file_ctx = BufferCtx::current(self.ctx.reborrow())
            .into_file()
            .ok_or(FindProjectRootError::NotInFile)?;

        let Some(project_root) = root_finder::Finder::new(OsFs)
            .find_root(file_ctx.path(), markers::Git)
            .await?
        else {
            return Err(FindProjectRootError::UnknownRoot {
                file_path: file_ctx.path().to_owned(),
            });
        };

        Ok(ConfirmStart { project_root, starter: self })
    }
}

impl ConfirmStart {
    async fn confirm_start(
        self,
    ) -> Result<ConnectToServer, ConfirmStartError> {
        Ok(ConnectToServer {
            project_root: self.project_root,
            starter: self.starter,
        })
    }
}

impl ConnectToServer {
    async fn connect_to_server(
        self,
    ) -> Result<Authenticate, ConnectToServerError> {
        collab_server::Io::connect()
            .await
            .map(|io| Authenticate {
                io,
                project_root: self.project_root,
                starter: self.starter,
            })
            .map_err(Into::into)
    }
}

impl Authenticate {
    async fn authenticate(
        self,
        auth_infos: AuthInfos,
    ) -> Result<StartSession, collab_server::client::AuthError> {
        self.io.authenticate(auth_infos.clone()).await.map(|authenticated| {
            StartSession {
                authenticated,
                auth_infos,
                project_root: self.project_root,
                starter: self.starter,
            }
        })
    }
}

impl StartSession {
    async fn start_session(self) -> Result<ReadReplica, StartSessionError> {
        self.authenticated
            .join(collab_server::client::JoinRequest::StartNewSession)
            .await
            .map(|joined| ReadReplica {
                local_peer: Peer::new(
                    joined.sender.peer_id(),
                    self.auth_infos.github_handle,
                ),
                joined,
                project_root: self.project_root,
                starter: self.starter,
            })
            .map_err(Into::into)
    }
}

impl ReadReplica {
    async fn read_replica(self) -> Result<RunSession, ReadReplicaError> {
        let (node_tx, node_rx) = flume::unbounded();
        recurse(
            self.project_root.clone(),
            NodeTx { inner: node_tx },
            self.starter.ctx.reborrow(),
        );

        let mut builder = ReplicaBuilder::new(self.local_peer.id());
        while let Ok(res) = node_rx.recv_async().await {
            let maybe_duplicate_path = match res? {
                Node::Dir { path } => {
                    let path_in_project = path
                        .strip_prefix(&self.project_root)
                        .expect("dir is a descendant of the project root");
                    builder
                        .push_directory(path_in_project)
                        .is_err()
                        .then_some(path)
                },
                Node::File { path, len } => {
                    let path_in_project = path
                        .strip_prefix(&self.project_root)
                        .expect("file is a descendant of the project root");
                    builder
                        .push_file(path_in_project, len)
                        .is_err()
                        .then_some(path)
                },
            };
            if let Some(path) = maybe_duplicate_path {
                return Err(ReadReplicaError::ReadDuplicate(path));
            }
        }

        Ok(RunSession {
            joined: self.joined,
            local_peer: self.local_peer,
            project_root: self.project_root,
            replica: builder.build(),
            starter: self.starter,
        })
    }
}

impl RunSession {
    async fn run_session(
        self,
    ) -> Result<(), RunSessionError<io::Error, io::Error>> {
        let collab_server::client::Joined {
            sender,
            receiver,
            session_id,
            peers: _,
        } = self.joined;

        let session = Session::new(NewSessionArgs {
            is_host: true,
            local_peer: self.local_peer,
            remote_peers: Peers::default(),
            project_root: self.project_root,
            replica: self.replica,
            session_id,
            neovim_ctx: self.starter.ctx,
        });

        let status = SessionStatus::InSession(session.project());
        self.starter.session_status.set(status);
        session.run(sender, receiver).await
    }
}

enum Node {
    Dir { path: AbsPathBuf },
    File { path: AbsPathBuf, len: u64 },
}

#[derive(Clone)]
struct NodeTx {
    inner: flume::Sender<Result<Node, ReadReplicaError>>,
}

impl NodeTx {
    fn send(&self, node: Result<Node, ReadReplicaError>) {
        self.inner.send(node).expect("receiver hasn't been dropped");
    }
}

#[allow(clippy::too_many_lines)]
fn recurse(mut dir_path: AbsPathBuf, node_tx: NodeTx, ctx: NeovimCtx<'_>) {
    ctx.spawn(|ctx| async move {
        let read_dir = async {
            let mut entries = match async_fs::read_dir(&dir_path).await {
                Ok(entries) => entries,
                Err(io_err) => {
                    return Err(ReadReplicaError::CouldntReadDir {
                        dir_path,
                        err: io_err,
                    });
                },
            };
            let mut is_dir_empty = true;
            while let Some(res) = entries.next().await {
                is_dir_empty = false;
                let dir_entry = match res {
                    Ok(dir_entry) => dir_entry,
                    Err(io_err) => {
                        return Err(ReadReplicaError::CouldntReadDir {
                            dir_path,
                            err: io_err,
                        })
                    },
                };
                let node_name_os = dir_entry.file_name();
                let node_name = match node_name_os
                    .to_str()
                    .and_then(|s| <&FsNodeName>::try_from(s).ok())
                {
                    Some(node_name) => node_name,
                    None => {
                        return Err(ReadReplicaError::NodeNameNotUtf8 {
                            parent_path: dir_path,
                            fs_node_name: node_name_os,
                        })
                    },
                };
                let node_type = match dir_entry.file_type().await {
                    Ok(node_type) => node_type,
                    Err(io_err) => {
                        return Err(ReadReplicaError::CouldntReadType {
                            fs_node_path: dir_path,
                            err: io_err,
                        })
                    },
                };
                dir_path.push(node_name);
                if node_type.is_file() {
                    let metadata = match dir_entry.metadata().await {
                        Ok(metadata) => metadata,
                        Err(io_err) => {
                            return Err(
                                ReadReplicaError::CouldntReadMetadata {
                                    fs_node_path: dir_path,
                                    err: io_err,
                                },
                            )
                        },
                    };
                    node_tx.send(Ok(Node::File {
                        path: dir_path.clone(),
                        len: metadata.len(),
                    }));
                } else if node_type.is_dir() {
                    recurse(dir_path.clone(), node_tx.clone(), ctx.clone());
                }
                dir_path.pop();
            }
            if is_dir_empty {
                node_tx.send(Ok(Node::Dir { path: dir_path }));
            }
            Ok(())
        };

        if let Err(err) = read_dir.await {
            node_tx.send(Err(err));
        }
    })
    .detach();
}

impl From<StartError> for DiagnosticMessage {
    fn from(err: StartError) -> Self {
        let mut message = Self::new();
        message.push_str(err.to_string());
        message
    }
}
