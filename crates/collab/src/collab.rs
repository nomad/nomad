use auth::AuthInfos;
use ed::module::{ApiCtx, Module};
use ed::notify::Name;
use ed::{Borrowed, Context, Shared};

use crate::backend::{CollabBackend, SessionId};
use crate::config::Config;
use crate::join::Join;
use crate::leave::{Leave, StopChannels};
use crate::project::{ProjectHandle, Projects};
use crate::start::Start;
use crate::yank::Yank;

/// TODO: docs.
pub struct Collab<B: CollabBackend> {
    pub(crate) auth_infos: Shared<Option<AuthInfos>>,
    pub(crate) config: Shared<Config>,
    pub(crate) projects: Projects<B>,
    pub(crate) stop_channels: StopChannels<B>,
}

impl<B: CollabBackend> Collab<B> {
    /// Returns a new instance of the [`Join`] action.
    pub fn join(&self) -> Join<B> {
        self.into()
    }

    /// Returns a new instance of the [`Leave`] action.
    pub fn leave(&self) -> Leave<B> {
        self.into()
    }

    /// Returns a handle to the project for the given [`SessionId`], if any.
    pub fn project(
        &self,
        session_id: SessionId<B>,
    ) -> Option<ProjectHandle<B>> {
        self.projects.get(session_id)
    }

    /// Returns a new instance of the [`Start`] action.
    pub fn start(&self) -> Start<B> {
        self.into()
    }

    /// Returns a new instance of the [`Yank`] action.
    pub fn yank(&self) -> Yank<B> {
        self.into()
    }
}

impl<B: CollabBackend> Module<B> for Collab<B> {
    const NAME: Name = "collab";

    type Config = Config;

    fn api(&self, ctx: &mut ApiCtx<B>) {
        ctx.with_command(self.join())
            .with_command(self.leave())
            .with_command(self.start())
            .with_command(self.yank())
            .with_function(self.join())
            .with_function(self.leave())
            .with_function(self.start())
            .with_function(self.yank());
    }

    fn on_new_config(
        &self,
        new_config: Self::Config,
        _ctx: &mut Context<B, Borrowed>,
    ) {
        self.config.set(new_config);
    }
}

impl<B: CollabBackend> From<&auth::Auth> for Collab<B> {
    fn from(auth: &auth::Auth) -> Self {
        Self {
            auth_infos: auth.infos().clone(),
            config: Default::default(),
            projects: Default::default(),
            stop_channels: Default::default(),
        }
    }
}
