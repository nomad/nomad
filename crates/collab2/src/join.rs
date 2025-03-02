//! TODO: docs.

use core::marker::PhantomData;

use auth::AuthInfos;
use collab_server::SessionId;
use collab_server::message::{FileContents, Message, ProjectRequest};
use eerie::Replica;
use futures_util::{SinkExt, StreamExt};
use nvimx2::action::AsyncAction;
use nvimx2::command::{Parse, ToCompletionFn};
use nvimx2::fs::{self, AbsPath};
use nvimx2::notify::Name;
use nvimx2::{AsyncCtx, Shared, notify};

use crate::backend::{CollabBackend, JoinArgs, JoinInfos};
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
    stop_channels: StopChannels,
}

impl<B: CollabBackend> AsyncAction<B> for Join<B> {
    const NAME: Name = "join";

    type Args = Parse<SessionId>;

    async fn call(
        &mut self,
        args: Self::Args,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), JoinError<B>> {
        let auth_infos = self
            .auth_infos
            .with(|infos| infos.as_ref().cloned())
            .ok_or_else(JoinError::user_not_logged_in)?;

        let join_args = JoinArgs {
            auth_infos: &auth_infos,
            session_id: args.into_inner(),
            server_address: &self.config.with(|c| c.server_address.clone()),
        };

        let mut join_infos = B::join_session(join_args, ctx)
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
        .join(&join_infos.project_name);

        let project_guard = self
            .projects
            .join_guard(project_root, join_infos.session_id)
            .map_err(JoinError::OverlappingProject)?;

        let ProjectResponse { buffered, file_contents, replica } =
            request_project(&mut join_infos)
                .await
                .map_err(JoinError::RequestProject)?;

        flush_project(
            &replica,
            &file_contents,
            project_guard.root(),
            ctx.fs(),
        )
        .await
        .map_err(JoinError::FlushProject)?;

        let project = project_guard.activate(NewProjectArgs {
            host: join_infos.host,
            local_peer: join_infos.local_peer,
            replica,
            remote_peers: join_infos.remote_peers,
        });

        let session = Session::new(NewSessionArgs {
            _project: project,
            server_rx: join_infos.server_rx,
            server_tx: join_infos.server_tx,
            stop_rx: self.stop_channels.insert(join_infos.session_id),
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

async fn request_project<B: CollabBackend>(
    join_infos: &mut JoinInfos<B>,
) -> Result<ProjectResponse, RequestProjectError<B>> {
    let request_from = join_infos
        .remote_peers
        .as_slice()
        .first()
        .expect("can't be empty")
        .id();

    join_infos
        .server_tx
        .send(Message::ProjectRequest(ProjectRequest {
            requested_by: join_infos.local_peer.clone(),
            request_from,
        }))
        .await
        .map_err(RequestProjectError::SendRequest)?;

    let mut buffered = Vec::new();

    let response = loop {
        let message = join_infos
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
        replica: Replica::decode(join_infos.local_peer.id(), response.replica),
    })
}

async fn flush_project<Fs: fs::Fs>(
    replica: &Replica,
    file_contents: &FileContents,
    project_root: &AbsPath,
    fs: Fs,
) -> Result<(), FlushProjectError<B>> {
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
    UserNotLoggedIn(UserNotLoggedInError<B>),
}

/// The type of error that can occur when requesting the state of the project
/// from another peer in a session fails.
pub enum RequestProjectError<B: CollabBackend> {
    /// TODO: docs.
    RecvResponse(B::ServerRxError),

    /// TODO: docs.
    SendRequest(B::ServerTxError),

    /// TODO: docs.
    SessionEnded,
}

/// TODO: docs.
pub enum FlushProjectError<Fs: fs::Fs> {
    /// TODO: docs.
    Todo(core::marker::PhantomData<Fs>),
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

impl<B: CollabBackend> JoinError<B> {
    /// Creates a new [`JoinError::UserNotLoggedIn`] variant.
    pub fn user_not_logged_in() -> Self {
        Self::UserNotLoggedIn(UserNotLoggedInError(PhantomData))
    }
}

impl<B> PartialEq for JoinError<B>
where
    B: CollabBackend,
    FlushProjectError<B::Fs>: PartialEq,
    B::DefaultDirForRemoteProjectsError: PartialEq,
    B::JoinSessionError: PartialEq,
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
            (RequestProject(_), RequestProject(_)) => todo!(),
            (UserNotLoggedIn(_), UserNotLoggedIn(_)) => true,
            _ => false,
        }
    }
}

impl<B: CollabBackend> notify::Error for JoinError<B> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            Self::DefaultDirForRemoteProjects(err) => err.to_message(),
            Self::FlushProject(_) => todo!(),
            Self::JoinSession(err) => err.to_message(),
            Self::OverlappingProject(err) => err.to_message(),
            Self::RequestProject(_) => todo!(),
            Self::UserNotLoggedIn(err) => err.to_message(),
        }
    }
}
