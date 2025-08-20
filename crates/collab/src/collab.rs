use auth::AuthInfos;
use editor::module::{ApiCtx, Module};
use editor::notify::Name;
use editor::{Borrowed, Context, Shared};

use crate::config::Config;
use crate::editors::{CollabEditor, SessionId};
use crate::join::{Join, JoinError};
use crate::leave::{self, Leave, LeaveError};
use crate::project::{ProjectHandle, Projects};
use crate::start::{Start, StartError};
use crate::yank::{Yank, YankError};

/// TODO: docs.
pub struct Collab<Ed: CollabEditor> {
    pub(crate) auth_infos: Shared<Option<AuthInfos>>,
    pub(crate) config: Shared<Config>,
    pub(crate) projects: Projects<Ed>,
    pub(crate) stop_channels: leave::StopChannels<Ed>,
}

impl<Ed: CollabEditor> Collab<Ed> {
    /// Calls the [`Join`] action with the given session ID.
    pub async fn join(
        &self,
        session_id: SessionId<Ed>,
        ctx: &mut Context<Ed>,
    ) -> Result<(), JoinError<Ed>> {
        Join::from(self).call_inner(session_id, ctx).await
    }

    /// Calls the [`Leave`] action.
    pub async fn leave(
        &self,
        ctx: &mut Context<Ed>,
    ) -> Result<(), LeaveError> {
        Leave::from(self).call_inner(ctx).await
    }

    /// Returns a handle to the project for the given [`SessionId`], if any.
    pub fn project(
        &self,
        session_id: SessionId<Ed>,
    ) -> Option<ProjectHandle<Ed>> {
        self.projects.get(session_id)
    }

    /// Calls the [`Start`] action.
    pub async fn start(
        &self,
        ctx: &mut Context<Ed>,
    ) -> Result<SessionId<Ed>, StartError<Ed>> {
        Start::from(self).call_inner(ctx).await
    }

    /// Calls the [`Yank`] action.
    pub async fn yank(
        &self,
        ctx: &mut Context<Ed>,
    ) -> Result<(), YankError<Ed>> {
        Yank::from(self).call_inner(ctx).await
    }
}

impl<Ed: CollabEditor> Module<Ed> for Collab<Ed> {
    const NAME: Name = "collab";

    type Config = Config;

    fn api(&self, ctx: &mut ApiCtx<Ed>) {
        ctx.with_command(Join::from(self))
            .with_command(Leave::from(self))
            .with_command(Start::from(self))
            .with_command(Yank::from(self))
            .with_function(Join::from(self))
            .with_function(Leave::from(self))
            .with_function(Start::from(self))
            .with_function(Yank::from(self));
    }

    fn on_new_config(
        &self,
        new_config: Self::Config,
        _ctx: &mut Context<Ed, Borrowed>,
    ) {
        self.config.set(new_config);
    }
}

impl<Ed: CollabEditor> From<&auth::Auth> for Collab<Ed> {
    fn from(auth: &auth::Auth) -> Self {
        Self {
            auth_infos: auth.infos().clone(),
            config: Default::default(),
            projects: Default::default(),
            stop_channels: Default::default(),
        }
    }
}
