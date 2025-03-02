//! TODO: docs.

use core::marker::PhantomData;

use auth::AuthInfos;
use collab_server::SessionId;
use nvimx2::action::AsyncAction;
use nvimx2::command::{Parse, ToCompletionFn};
use nvimx2::fs::Directory;
use nvimx2::notify::Name;
use nvimx2::{AsyncCtx, Shared, notify};

use crate::Project;
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

impl<B: CollabBackend> Join<B> {
    async fn request_project(
        &self,
        _join_infos: &mut JoinInfos<B>,
    ) -> Result<Project<B>, RequestProjectError<B>> {
        todo!();
    }
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

        let project = self
            .request_project(&mut join_infos)
            .await
            .map_err(JoinError::RequestProject)?;

        let project_root = B::root_for_remote_project(&project, ctx)
            .await
            .map_err(JoinError::RootForRemoteProject)?;

        project.flush(&project_root, ctx.fs()).await;

        let project = self
            .projects
            .insert(NewProjectArgs {
                replica: todo!(),
                root: project_root.path().to_owned(),
                session_id: join_infos.session_id,
            })
            .map_err(JoinError::OverlappingProject)?;

        let session = Session::new(NewSessionArgs {
            _is_host: false,
            _local_peer: join_infos.local_peer,
            _project: project,
            _remote_peers: join_infos.remote_peers,
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

/// The type of error that can occur when [`Join`]ing a session fails.
#[derive(derive_more::Debug)]
#[debug(bound(B: CollabBackend))]
pub enum JoinError<B: CollabBackend> {
    /// TODO: docs.
    JoinSession(B::JoinSessionError),

    /// TODO: docs.
    OverlappingProject(OverlappingProjectError),

    /// TODO: docs.
    RequestProject(RequestProjectError<B>),

    /// TODO: docs.
    RootForRemoteProject(B::RootForRemoteProjectError),

    /// TODO: docs.
    UserNotLoggedIn(UserNotLoggedInError<B>),
}

/// The type of error that can occur when requesting the state of the project
/// from another peer in a session fails.
pub enum RequestProjectError<B: CollabBackend> {
    /// TODO: docs.
    Todo(core::marker::PhantomData<B>),
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
    B::JoinSessionError: PartialEq,
    B::RootForRemoteProjectError: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        use JoinError::*;

        match (self, other) {
            (JoinSession(l), JoinSession(r)) => l == r,
            (OverlappingProject(l), OverlappingProject(r)) => l == r,
            (RequestProject(_), RequestProject(_)) => todo!(),
            (RootForRemoteProject(l), RootForRemoteProject(r)) => l == r,
            (UserNotLoggedIn(_), UserNotLoggedIn(_)) => true,
            _ => false,
        }
    }
}

impl<B: CollabBackend> notify::Error for JoinError<B> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            Self::JoinSession(err) => err.to_message(),
            Self::OverlappingProject(err) => err.to_message(),
            Self::RequestProject(_) => todo!(),
            Self::RootForRemoteProject(err) => err.to_message(),
            Self::UserNotLoggedIn(err) => err.to_message(),
        }
    }
}
