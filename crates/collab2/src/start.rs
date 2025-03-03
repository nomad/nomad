//! TODO: docs.

use core::fmt;
use core::marker::PhantomData;

use auth::AuthInfos;
use nvimx2::action::AsyncAction;
use nvimx2::command::ToCompletionFn;
use nvimx2::notify::{self, Name};
use nvimx2::{AsyncCtx, Shared};

use crate::backend::{CollabBackend, StartArgs};
use crate::collab::Collab;
use crate::config::Config;
use crate::leave::StopChannels;
use crate::project::{NewProjectArgs, OverlappingProjectError, Projects};
use crate::session::{NewSessionArgs, Session};

/// The `Action` used to start a new collaborative editing session.
pub struct Start<B: CollabBackend> {
    auth_infos: Shared<Option<AuthInfos>>,
    config: Shared<Config>,
    projects: Projects<B>,
    stop_channels: StopChannels,
}

impl<B: CollabBackend> AsyncAction<B> for Start<B> {
    const NAME: Name = "start";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), StartError<B>> {
        let auth_infos = self
            .auth_infos
            .with(|infos| infos.as_ref().cloned())
            .ok_or_else(StartError::user_not_logged_in)?;

        let buffer_id = ctx.with_ctx(|ctx| {
            ctx.current_buffer()
                .map(|buf| buf.id())
                .ok_or_else(StartError::no_buffer_focused)
        })?;

        let project_root = B::search_project_root(buffer_id, ctx)
            .await
            .map_err(StartError::SearchProjectRoot)?;

        let project_guard = self
            .projects
            .new_guard(project_root)
            .map_err(StartError::OverlappingProject)?;

        if !B::confirm_start(project_guard.root(), ctx).await {
            return Ok(());
        }

        let project_name = project_guard
            .root()
            .node_name()
            .ok_or(StartError::ProjectRootIsFsRoot)?;

        let start_args = StartArgs {
            auth_infos: &auth_infos,
            project_name,
            server_address: &self.config.with(|c| c.server_address.clone()),
        };

        let sesh_infos = B::start_session(start_args, ctx)
            .await
            .map_err(StartError::StartSession)?;

        let replica = B::read_replica(
            sesh_infos.local_peer.id(),
            project_guard.root(),
            ctx,
        )
        .await
        .map_err(StartError::ReadReplica)?;

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

/// The type of error that can occur when [`Start`]ing a session fails.
#[derive(derive_more::Debug)]
#[debug(bound(B: CollabBackend))]
pub enum StartError<B: CollabBackend> {
    /// TODO: docs.
    NoBufferFocused(NoBufferFocusedError<B>),

    /// TODO: docs.
    OverlappingProject(OverlappingProjectError),

    /// TODO: docs.
    ProjectRootIsFsRoot,

    /// TODO: docs.
    ReadReplica(B::ReadReplicaError),

    /// TODO: docs.
    SearchProjectRoot(B::SearchProjectRootError),

    /// TODO: docs.
    StartSession(B::StartSessionError),

    /// TODO: docs.
    UserNotLoggedIn(UserNotLoggedInError<B>),
}

/// TODO: docs.
pub struct NoBufferFocusedError<B>(PhantomData<B>);

/// TODO: docs.
pub struct UserNotLoggedInError<B>(pub(crate) PhantomData<B>);

impl<B: CollabBackend> Clone for Start<B> {
    fn clone(&self) -> Self {
        Self {
            auth_infos: self.auth_infos.clone(),
            config: self.config.clone(),
            stop_channels: self.stop_channels.clone(),
            projects: self.projects.clone(),
        }
    }
}

impl<B: CollabBackend> From<&Collab<B>> for Start<B> {
    fn from(collab: &Collab<B>) -> Self {
        Self {
            auth_infos: collab.auth_infos.clone(),
            config: collab.config.clone(),
            projects: collab.projects.clone(),
            stop_channels: collab.stop_channels.clone(),
        }
    }
}

impl<B: CollabBackend> ToCompletionFn<B> for Start<B> {
    fn to_completion_fn(&self) {}
}

impl<B: CollabBackend> StartError<B> {
    /// Creates a new [`StartError::NoBufferFocused`] variant.
    pub fn no_buffer_focused() -> Self {
        Self::NoBufferFocused(NoBufferFocusedError(PhantomData))
    }

    /// Creates a new [`StartError::UserNotLoggedIn`] variant.
    pub fn user_not_logged_in() -> Self {
        Self::UserNotLoggedIn(UserNotLoggedInError(PhantomData))
    }
}

impl<B> PartialEq for StartError<B>
where
    B: CollabBackend,
    B::ReadReplicaError: PartialEq,
    B::SearchProjectRootError: PartialEq,
    B::StartSessionError: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        use StartError::*;

        match (self, other) {
            (NoBufferFocused(_), NoBufferFocused(_)) => true,
            (OverlappingProject(l), OverlappingProject(r)) => l == r,
            (ProjectRootIsFsRoot, ProjectRootIsFsRoot) => true,
            (ReadReplica(l), ReadReplica(r)) => l == r,
            (SearchProjectRoot(l), SearchProjectRoot(r)) => l == r,
            (StartSession(l), StartSession(r)) => l == r,
            (UserNotLoggedIn(_), UserNotLoggedIn(_)) => true,
            _ => false,
        }
    }
}

impl<B: CollabBackend> notify::Error for StartError<B> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            Self::NoBufferFocused(err) => err.to_message(),
            Self::OverlappingProject(err) => err.to_message(),
            Self::ProjectRootIsFsRoot => (
                notify::Level::Error,
                notify::Message::from_str(
                    "cannot start a new collaborative editing session at the \
                     root of the filesystem",
                ),
            ),
            Self::ReadReplica(err) => err.to_message(),
            Self::SearchProjectRoot(err) => err.to_message(),
            Self::StartSession(err) => err.to_message(),
            Self::UserNotLoggedIn(err) => err.to_message(),
        }
    }
}

impl<B> fmt::Debug for NoBufferFocusedError<B> {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl<B> fmt::Debug for UserNotLoggedInError<B> {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl<B> notify::Error for NoBufferFocusedError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Off, notify::Message::new())
    }
}

impl<B> notify::Error for UserNotLoggedInError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Off, notify::Message::new())
    }
}

#[cfg(feature = "neovim")]
mod neovim_error_impls {
    use nvimx2::neovim::Neovim;

    use super::*;

    impl notify::Error for NoBufferFocusedError<Neovim> {
        fn to_message(&self) -> (notify::Level, notify::Message) {
            let msg = "couldn't determine path to project root. Either move \
                       the cursor to a text buffer, or pass one explicitly";
            (notify::Level::Error, notify::Message::from_str(msg))
        }
    }

    impl notify::Error for UserNotLoggedInError<Neovim> {
        fn to_message(&self) -> (notify::Level, notify::Message) {
            let mut msg = notify::Message::from_str(
                "need to be logged in to collaborate. You can log in by \
                 executing ",
            );
            msg.push_expected(":Mad login");
            (notify::Level::Error, msg)
        }
    }
}
