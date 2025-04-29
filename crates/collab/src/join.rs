//! TODO: docs.

use core::fmt;
use std::io;

use auth::AuthInfos;
use collab_project::Project;
use collab_project::fs::{DirectoryId, File as ProjectFile, FileId, Node};
use collab_server::message::{Message, Peer, ProjectRequest};
use collab_server::{SessionIntent, client};
use ed::action::AsyncAction;
use ed::command::ToCompletionFn;
use ed::fs::{self, AbsPath, Directory, File};
use ed::notify::Name;
use ed::{AsyncCtx, Shared, notify};
use futures_util::{AsyncReadExt, SinkExt, StreamExt, future, stream};

use crate::backend::{CollabBackend, SessionId, Welcome};
use crate::collab::Collab;
use crate::config::Config;
use crate::event_stream::EventStream;
use crate::leave::StopChannels;
use crate::project::{
    IdMaps,
    NewProjectArgs,
    OverlappingProjectError,
    Projects,
};
use crate::session::Session;
use crate::start::UserNotLoggedInError;

/// The `Action` used to join an existing collaborative editing session.
#[derive(cauchy::Clone)]
pub struct Join<B: CollabBackend> {
    auth_infos: Shared<Option<AuthInfos>>,
    config: Shared<Config>,
    projects: Projects<B>,
    stop_channels: StopChannels<B>,
}

impl<B: CollabBackend> AsyncAction<B> for Join<B> {
    const NAME: Name = "join";

    type Args = SessionId<B>;

    #[allow(clippy::too_many_lines)]
    async fn call(
        &mut self,
        session_id: Self::Args,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), JoinError<B>> {
        let auth_infos =
            self.auth_infos.cloned().ok_or(JoinError::UserNotLoggedIn)?;

        let server_addr = self.config.with(|c| c.server_address.clone());

        let (reader, writer) = B::connect_to_server(server_addr, ctx)
            .await
            .map_err(JoinError::ConnectToServer)?
            .split();

        let github_handle = auth_infos.handle().clone();

        let knock = collab_server::Knock::<B::ServerConfig> {
            auth_infos: auth_infos.into(),
            session_intent: SessionIntent::JoinExisting(session_id),
        };

        let mut welcome = client::Knocker::new(reader, writer)
            .knock(knock)
            .await
            .map_err(JoinError::Knock)?;

        let project_root = match self
            .config
            .with(|c| c.store_remote_projects_under.clone())
        {
            Some(remote_dir) => remote_dir,
            None => B::default_dir_for_remote_projects(ctx)
                .await
                .map_err(JoinError::DefaultDirForRemoteProjects)?,
        }
        .join(&welcome.project_name);

        let project_guard = self
            .projects
            .new_guard(project_root)
            .map_err(JoinError::OverlappingProject)?;

        let local_peer = Peer { id: welcome.peer_id, github_handle };

        let ProjectResponse { buffered, project } =
            request_project::<B>(local_peer.clone(), &mut welcome)
                .await
                .map_err(JoinError::RequestProject)?;

        let (event_stream, id_maps) =
            write_project(&project, project_guard.root(), ctx)
                .await
                .map_err(JoinError::WriteProject)?;

        let project_handle = project_guard.activate(NewProjectArgs {
            id_maps,
            host_id: welcome.host_id,
            peer_handle: local_peer.github_handle.clone(),
            project,
            remote_peers: welcome.other_peers,
            session_id: welcome.session_id,
        });

        project_handle.with_mut(|proj| {
            for message in buffered {
                proj.integrate_message(message, ctx);
            }
        });

        let session = Session {
            event_stream,
            message_rx: welcome.rx,
            message_tx: welcome.tx,
            project_handle,
            stop_rx: self.stop_channels.insert(welcome.session_id),
        };

        ctx.spawn_local(async move |ctx| {
            if let Err(err) = session.run(ctx).await {
                ctx.emit_err(err);
            }
        })
        .detach();

        Ok(())
    }
}

/// TODO: docs.
async fn write_project<B: CollabBackend>(
    project: &Project,
    root_path: &AbsPath,
    ctx: &mut AsyncCtx<'_, B>,
) -> Result<(EventStream<B>, IdMaps<B>), WriteProjectError<B::Fs>> {
    todo!();
}

struct ProjectResponse {
    buffered: Vec<Message>,
    project: Project,
}

struct ProjectTree<'a> {
    project: &'a Project,
}

async fn request_project<B: CollabBackend>(
    local_peer: Peer,
    welcome: &mut Welcome<B>,
) -> Result<ProjectResponse, RequestProjectError> {
    let local_id = local_peer.id;

    let request = ProjectRequest {
        requested_by: local_peer,
        request_from: welcome
            .other_peers
            .as_slice()
            .first()
            .expect("can't be empty")
            .id,
    };

    welcome
        .tx
        .send(Message::ProjectRequest(request))
        .await
        .map_err(RequestProjectError::SendRequest)?;

    let mut buffered = Vec::new();

    let response = loop {
        let message = welcome
            .rx
            .next()
            .await
            .ok_or(RequestProjectError::SessionEnded)?
            .map_err(RequestProjectError::RecvResponse)?;

        match message {
            Message::ProjectResponse(response) => break response,
            other => buffered.push(other),
        }
    };

    Ok(ProjectResponse {
        buffered,
        project: Project::from_state(local_id, *response.project),
    })
}

impl<'a> ProjectTree<'a> {
    async fn flush<Fs: fs::Fs>(
        &self,
        flush_under: &AbsPath,
        fs: Fs,
    ) -> Result<(), WriteProjectError<Fs>> {
        if let Some(node) = fs
            .node_at_path(flush_under)
            .await
            .map_err(WriteProjectError::GetNodeAtRoot)?
        {
            node.delete().await.map_err(WriteProjectError::DeleteNodeAtRoot)?
        }

        let root = fs
            .create_directory(flush_under)
            .await
            .map_err(WriteProjectError::GetOrCreateRoot)?;

        root.clear().await.map_err(WriteProjectError::ClearRoot)?;

        let root_id = self.project.root().id();

        self.flush_directory(root_id, &root).await
    }

    async fn flush_directory<Fs: fs::Fs>(
        &self,
        dir_id: DirectoryId,
        dir: &Fs::Directory,
    ) -> Result<(), WriteProjectError<Fs>> {
        let parent = self.project.directory(dir_id).expect("ID is valid");

        let flush_dirs = parent
            .children()
            .filter_map(|node| match node {
                Node::Directory(dir) => Some(dir),
                Node::File(_) => None,
            })
            .map(|child_dir| {
                let child_id = child_dir.id();
                let child_name =
                    child_dir.name().expect("child can't be root");
                async move {
                    let child = dir
                        .create_directory(child_name)
                        .await
                        .map_err(WriteProjectError::CreateDirectory)?;
                    self.flush_directory::<Fs>(child_id, &child).await
                }
            });

        let flush_files = parent
            .children()
            .filter_map(|node| match node {
                Node::Directory(_) => None,
                Node::File(file) => Some(file),
            })
            .map(|child_file| {
                let child_id = child_file.id();
                let child_name = child_file.name();
                async move {
                    let mut child = dir
                        .create_file(child_name)
                        .await
                        .map_err(WriteProjectError::CreateFile)?;
                    self.flush_file::<Fs>(child_id, &mut child).await
                }
            });

        let mut flush_children = flush_dirs
            .map(future::Either::Left)
            .chain(flush_files.map(future::Either::Right))
            .collect::<stream::FuturesOrdered<_>>();

        while let Some(res) = flush_children.next().await {
            res?;
        }

        Ok(())
    }

    async fn flush_file<Fs: fs::Fs>(
        &self,
        file_id: FileId,
        file: &mut Fs::File,
    ) -> Result<(), WriteProjectError<Fs>> {
        match self.project.file(file_id).expect("ID is valid") {
            ProjectFile::Binary(binary_file) => {
                file.write(binary_file.contents()).await
            },
            ProjectFile::Symlink(symlink_file) => {
                file.write(symlink_file.target_path()).await
            },
            ProjectFile::Text(text_file) => {
                file.write(text_file.contents().to_string()).await
            },
        }
        .map_err(WriteProjectError::WriteToFile)
    }

    fn new(project: &'a Project) -> Self {
        Self { project }
    }
}

/// The type of error that can occur when [`Join`]ing a session fails.
#[derive(cauchy::Debug, cauchy::PartialEq)]
pub enum JoinError<B: CollabBackend> {
    /// TODO: docs.
    ConnectToServer(B::ConnectToServerError),

    /// TODO: docs.
    DefaultDirForRemoteProjects(B::DefaultDirForRemoteProjectsError),

    /// TODO: docs.
    Knock(client::KnockError<B::ServerConfig>),

    /// TODO: docs.
    OverlappingProject(OverlappingProjectError),

    /// TODO: docs.
    RequestProject(RequestProjectError),

    /// TODO: docs.
    UserNotLoggedIn,

    /// TODO: docs.
    WriteProject(WriteProjectError<B::Fs>),
}

/// The type of error that can occur when requesting the state of the project
/// from another peer in a session fails.
#[derive(Debug, cauchy::PartialEq)]
pub enum RequestProjectError {
    /// TODO: docs.
    RecvResponse(#[partial_eq(skip)] client::ClientRxError),

    /// TODO: docs.
    SendRequest(#[partial_eq(skip)] io::Error),

    /// TODO: docs.
    SessionEnded,
}

/// TODO: docs.
#[derive(cauchy::Debug, cauchy::PartialEq)]
pub enum WriteProjectError<Fs: fs::Fs> {
    /// TODO: docs.
    CreateDirectory(<Fs::Directory as fs::Directory>::CreateDirectoryError),

    /// TODO: docs.
    CreateFile(<Fs::Directory as fs::Directory>::CreateFileError),

    /// TODO: docs.
    ClearRoot(<Fs::Directory as fs::Directory>::ClearError),

    /// TODO: docs.
    DeleteNodeAtRoot(fs::NodeDeleteError<Fs>),

    /// TODO: docs.
    GetOrCreateRoot(Fs::CreateDirectoryError),

    /// TODO: docs.
    GetNodeAtRoot(Fs::NodeAtPathError),

    /// TODO: docs.
    WriteToFile(<Fs::File as fs::File>::WriteError),
}

impl<B: CollabBackend> From<&Collab<B>> for Join<B> {
    fn from(collab: &Collab<B>) -> Self {
        Self {
            auth_infos: collab.auth_infos.clone(),
            config: collab.config.clone(),
            projects: collab.projects.clone(),
            stop_channels: collab.stop_channels.clone(),
        }
    }
}

impl<B: CollabBackend> ToCompletionFn<B> for Join<B> {
    fn to_completion_fn(&self) {}
}

impl<B: CollabBackend> notify::Error for JoinError<B> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            Self::ConnectToServer(err) => err.to_message(),
            Self::DefaultDirForRemoteProjects(err) => err.to_message(),
            Self::WriteProject(err) => err.to_message(),
            Self::Knock(_err) => todo!(),
            Self::OverlappingProject(err) => err.to_message(),
            Self::RequestProject(err) => err.to_message(),
            Self::UserNotLoggedIn => {
                UserNotLoggedInError::<B>::new().to_message()
            },
        }
    }
}

impl<Fs: fs::Fs> notify::Error for WriteProjectError<Fs> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        let err: &dyn fmt::Display = match self {
            Self::CreateDirectory(err) => err,
            Self::CreateFile(err) => err,
            Self::ClearRoot(err) => err,
            Self::DeleteNodeAtRoot(err) => err,
            Self::GetOrCreateRoot(err) => err,
            Self::GetNodeAtRoot(err) => err,
            Self::WriteToFile(err) => err,
        };
        (notify::Level::Error, notify::Message::from_display(err))
    }
}

impl notify::Error for RequestProjectError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            Self::RecvResponse(_err) => todo!(),
            Self::SendRequest(_err) => todo!(),
            Self::SessionEnded => (
                notify::Level::Error,
                notify::Message::from_str(
                    "session ended before we could join it",
                ),
            ),
        }
    }
}
