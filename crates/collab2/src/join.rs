//! TODO: docs.

use core::fmt;

use auth::AuthInfos;
use collab_server::message::{FileContents, Message, ProjectRequest};
use eerie::{DirectoryId, FileId, Replica};
use futures_util::{SinkExt, StreamExt, future, stream};
use nvimx2::action::AsyncAction;
use nvimx2::command::{Parse, ToCompletionFn};
use nvimx2::fs::{self, AbsPath, Directory, File};
use nvimx2::notify::Name;
use nvimx2::{AsyncCtx, Shared, notify};

use crate::backend::{CollabBackend, JoinArgs, SessionInfos};
use crate::collab::Collab;
use crate::config::Config;
use crate::leave::StopChannels;
use crate::project::{NewProjectArgs, OverlappingProjectError, Projects};
use crate::session::{NewSessionArgs, Session};
use crate::start::UserNotLoggedInError;

/// The `Action` used to join an existing collaborative editing session.
pub struct Join<B: CollabBackend> {
    auth_infos: Shared<Option<AuthInfos>>,
    config: Shared<Config>,
    projects: Projects<B>,
    stop_channels: StopChannels<B>,
}

impl<B: CollabBackend> AsyncAction<B> for Join<B> {
    const NAME: Name = "join";

    type Args = Parse<B::SessionId>;

    async fn call(
        &mut self,
        args: Self::Args,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), JoinError<B>> {
        let auth_infos = self
            .auth_infos
            .with(|infos| infos.as_ref().cloned())
            .ok_or(JoinError::UserNotLoggedIn)?;

        let join_args = JoinArgs {
            auth_infos: &auth_infos,
            session_id: args.into_inner(),
            server_address: &self.config.with(|c| c.server_address.clone()),
        };

        let mut sesh_infos = B::join_session(join_args, ctx)
            .await
            .map_err(JoinError::JoinSession)?;

        let project_root = match self
            .config
            .with(|c| c.store_remote_projects_under.clone())
        {
            Some(path) => path,
            None => B::default_dir_for_remote_projects(ctx)
                .await
                .map_err(JoinError::DefaultDirForRemoteProjects)?,
        }
        .join(&sesh_infos.project_name);

        let project_guard = self
            .projects
            .new_guard(project_root)
            .map_err(JoinError::OverlappingProject)?;

        let ProjectResponse { buffered, file_contents, replica } =
            request_project(&mut sesh_infos)
                .await
                .map_err(JoinError::RequestProject)?;

        ProjectTree::new(&replica, &file_contents)
            .flush(project_guard.root(), ctx.fs())
            .await
            .map_err(JoinError::FlushProject)?;

        let project_handle = project_guard.activate(NewProjectArgs {
            host_id: sesh_infos.host_id,
            local_peer: sesh_infos.local_peer,
            replica,
            remote_peers: sesh_infos.remote_peers,
            session_id: sesh_infos.session_id,
        });

        let session = Session::new(NewSessionArgs {
            project_handle,
            server_rx: sesh_infos.server_rx,
            server_tx: sesh_infos.server_tx,
            stop_rx: self.stop_channels.insert(sesh_infos.session_id),
        });

        ctx.spawn_local(async move |ctx| {
            if let Err(err) = session.run(ctx).await {
                ctx.emit_err(err);
            }
        })
        .detach();

        Ok(())
    }
}

struct ProjectResponse {
    buffered: Vec<Message>,
    file_contents: FileContents,
    replica: Replica,
}

struct ProjectTree<'a> {
    file_contents: &'a FileContents,
    replica: &'a Replica,
}

async fn request_project<B: CollabBackend>(
    infos: &mut SessionInfos<B>,
) -> Result<ProjectResponse, RequestProjectError<B>> {
    let request = ProjectRequest {
        requested_by: infos.local_peer.clone(),
        request_from: infos
            .remote_peers
            .as_slice()
            .first()
            .expect("can't be empty")
            .id(),
    };

    infos
        .server_tx
        .send(Message::ProjectRequest(request))
        .await
        .map_err(RequestProjectError::SendRequest)?;

    let mut buffered = Vec::new();

    let response = loop {
        let message = infos
            .server_rx
            .next()
            .await
            .ok_or(RequestProjectError::SessionEnded)?
            .map_err(RequestProjectError::RecvResponse)?;

        match message {
            Message::ProjectResponse(response) => break *response,
            other => buffered.push(other),
        }
    };

    Ok(ProjectResponse {
        buffered,
        file_contents: response.file_contents,
        replica: Replica::decode(infos.local_peer.id(), response.replica),
    })
}

impl<'a> ProjectTree<'a> {
    async fn flush<Fs: fs::Fs>(
        &self,
        flush_under: &AbsPath,
        fs: Fs,
    ) -> Result<(), FlushProjectError<Fs>> {
        if let Some(node) = fs
            .node_at_path(flush_under)
            .await
            .map_err(FlushProjectError::GetNodeAtRoot)?
        {
            node.delete().await.map_err(FlushProjectError::DeleteNodeAtRoot)?
        }

        let root = fs
            .create_directory(flush_under)
            .await
            .map_err(FlushProjectError::GetOrCreateRoot)?;

        root.clear().await.map_err(FlushProjectError::ClearRoot)?;

        let root_id = self.replica.root().id();

        self.flush_directory(root_id, &root).await
    }

    async fn flush_directory<Fs: fs::Fs>(
        &self,
        dir_id: DirectoryId,
        dir: &Fs::Directory,
    ) -> Result<(), FlushProjectError<Fs>> {
        let parent = self.replica.directory(dir_id).expect("ID is valid");

        let flush_dirs = parent.child_directories().map(|child| {
            let child_id = child.id();
            let child_name = child.name().expect("child can't be root");
            async move {
                let child = dir
                    .create_directory(child_name)
                    .await
                    .map_err(FlushProjectError::CreateDirectory)?;
                self.flush_directory::<Fs>(child_id, &child).await
            }
        });

        let flush_files = parent.child_files().map(|child| {
            let child_id = child.id();
            let child_name = child.name();
            async move {
                let child = dir
                    .create_file(child_name)
                    .await
                    .map_err(FlushProjectError::CreateFile)?;
                self.flush_file::<Fs>(child_id, &child).await
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
        file: &Fs::File,
    ) -> Result<(), FlushProjectError<Fs>> {
        let contents = self.file_contents.get(file_id).expect("ID is valid");
        file.write(contents).await.map_err(FlushProjectError::WriteToFile)
    }

    fn new(replica: &'a Replica, file_contents: &'a FileContents) -> Self {
        Self { file_contents, replica }
    }
}

/// The type of error that can occur when [`Join`]ing a session fails.
#[derive(derive_more::Debug)]
#[debug(bound(B: CollabBackend))]
pub enum JoinError<B: CollabBackend> {
    /// TODO: docs.
    DefaultDirForRemoteProjects(B::DefaultDirForRemoteProjectsError),

    /// TODO: docs.
    FlushProject(FlushProjectError<B::Fs>),

    /// TODO: docs.
    JoinSession(B::JoinSessionError),

    /// TODO: docs.
    OverlappingProject(OverlappingProjectError),

    /// TODO: docs.
    RequestProject(RequestProjectError<B>),

    /// TODO: docs.
    UserNotLoggedIn,
}

/// The type of error that can occur when requesting the state of the project
/// from another peer in a session fails.
#[derive(derive_more::Debug)]
#[debug(bound(B: CollabBackend))]
pub enum RequestProjectError<B: CollabBackend> {
    /// TODO: docs.
    RecvResponse(B::ServerRxError),

    /// TODO: docs.
    SendRequest(B::ServerTxError),

    /// TODO: docs.
    SessionEnded,
}

/// TODO: docs.
#[derive(derive_more::Debug)]
#[debug(bound(Fs: fs::Fs))]
pub enum FlushProjectError<Fs: fs::Fs> {
    /// TODO: docs.
    CreateDirectory(<Fs::Directory as fs::Directory>::CreateDirectoryError),

    /// TODO: docs.
    CreateFile(<Fs::Directory as fs::Directory>::CreateFileError),

    /// TODO: docs.
    ClearRoot(<Fs::Directory as fs::Directory>::ClearError),

    /// TODO: docs.
    DeleteNodeAtRoot(fs::DeleteNodeError<Fs>),

    /// TODO: docs.
    GetOrCreateRoot(Fs::CreateDirectoryError),

    /// TODO: docs.
    GetNodeAtRoot(Fs::NodeAtPathError),

    /// TODO: docs.
    WriteToFile(<Fs::File as fs::File>::WriteError),
}

impl<B: CollabBackend> Clone for Join<B> {
    fn clone(&self) -> Self {
        Self {
            auth_infos: self.auth_infos.clone(),
            config: self.config.clone(),
            stop_channels: self.stop_channels.clone(),
            projects: self.projects.clone(),
        }
    }
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

impl<B> PartialEq for JoinError<B>
where
    B: CollabBackend,
    B::DefaultDirForRemoteProjectsError: PartialEq,
    B::JoinSessionError: PartialEq,
    FlushProjectError<B::Fs>: PartialEq,
    RequestProjectError<B>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        use JoinError::*;

        match (self, other) {
            (
                DefaultDirForRemoteProjects(l),
                DefaultDirForRemoteProjects(r),
            ) => l == r,
            (FlushProject(l), FlushProject(r)) => l == r,
            (JoinSession(l), JoinSession(r)) => l == r,
            (OverlappingProject(l), OverlappingProject(r)) => l == r,
            (RequestProject(l), RequestProject(r)) => l == r,
            (UserNotLoggedIn, UserNotLoggedIn) => true,
            _ => false,
        }
    }
}

impl<B: CollabBackend> notify::Error for JoinError<B> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            Self::DefaultDirForRemoteProjects(err) => err.to_message(),
            Self::FlushProject(err) => err.to_message(),
            Self::JoinSession(err) => err.to_message(),
            Self::OverlappingProject(err) => err.to_message(),
            Self::RequestProject(err) => err.to_message(),
            Self::UserNotLoggedIn => {
                UserNotLoggedInError::<B>::new().to_message()
            },
        }
    }
}

impl<Fs: fs::Fs> PartialEq for FlushProjectError<Fs>
where
    <Fs::Directory as fs::Directory>::CreateDirectoryError: PartialEq,
    <Fs::Directory as fs::Directory>::CreateFileError: PartialEq,
    <Fs::Directory as fs::Directory>::ClearError: PartialEq,
    fs::DeleteNodeError<Fs>: PartialEq,
    Fs::CreateDirectoryError: PartialEq,
    Fs::NodeAtPathError: PartialEq,
    <Fs::File as fs::File>::WriteError: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        use FlushProjectError::*;

        match (self, other) {
            (CreateDirectory(l), CreateDirectory(r)) => l == r,
            (CreateFile(l), CreateFile(r)) => l == r,
            (ClearRoot(l), ClearRoot(r)) => l == r,
            (DeleteNodeAtRoot(l), DeleteNodeAtRoot(r)) => l == r,
            (GetOrCreateRoot(l), GetOrCreateRoot(r)) => l == r,
            (GetNodeAtRoot(l), GetNodeAtRoot(r)) => l == r,
            (WriteToFile(l), WriteToFile(r)) => l == r,
            _ => false,
        }
    }
}

impl<Fs: fs::Fs> notify::Error for FlushProjectError<Fs> {
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

impl<B: CollabBackend> PartialEq for RequestProjectError<B>
where
    B::ServerRxError: PartialEq,
    B::ServerTxError: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        use RequestProjectError::*;

        match (self, other) {
            (RecvResponse(l), RecvResponse(r)) => l == r,
            (SendRequest(l), SendRequest(r)) => l == r,
            (SessionEnded, SessionEnded) => true,
            _ => false,
        }
    }
}

impl<B: CollabBackend> notify::Error for RequestProjectError<B> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            Self::RecvResponse(err) => err.to_message(),
            Self::SendRequest(err) => err.to_message(),
            Self::SessionEnded => (
                notify::Level::Error,
                notify::Message::from_str(
                    "session ended before we could join it",
                ),
            ),
        }
    }
}
