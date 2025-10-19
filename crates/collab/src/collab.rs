use auth::AuthState;
use collab_types::PeerHandle;
use editor::context::Borrowed;
use editor::module::{ApiCtx, Module};
use editor::{Context, Shared};

use crate::config::Config;
use crate::copy_id::{CopyId, CopyIdError};
use crate::editors::{CollabEditor, SessionId};
use crate::join::{Join, JoinError};
use crate::jump::{Jump, JumpError};
use crate::leave::{self, Leave, LeaveError};
use crate::pause::{Pause, PauseError};
use crate::progress::ProgressReporter;
use crate::resume::{Resume, ResumeError};
use crate::session::{SessionInfos, Sessions};
use crate::start::{Start, StartError};

/// TODO: docs.
pub struct Collab<Ed: CollabEditor> {
    pub(crate) auth_state: AuthState,
    pub(crate) config: Shared<Config>,
    pub(crate) sessions: Sessions<Ed>,
    pub(crate) stop_channels: leave::StopChannels<Ed>,
}

impl<Ed: CollabEditor> Collab<Ed> {
    /// Calls the [`CopyId`] action.
    pub async fn copy_id(
        &self,
        ctx: &mut Context<Ed>,
    ) -> Result<(), CopyIdError<Ed>> {
        CopyId::from(self).call_inner(ctx).await
    }

    /// Calls the [`Join`] action with the given session ID.
    pub async fn join(
        &self,
        session_id: SessionId<Ed>,
        ctx: &mut Context<Ed>,
    ) -> Result<SessionInfos<Ed>, JoinError<Ed>> {
        let mut reporter =
            <Ed::ProgressReporter as ProgressReporter<_, Join<_>>>::new(ctx);
        Join::from(self).call_inner(session_id, &mut reporter, ctx).await
    }

    /// Calls the [`Jump`] action.
    pub async fn jump(
        &self,
        peer_handle: PeerHandle,
    ) -> Result<(), JumpError<Ed>> {
        Jump::from(self).call_inner(peer_handle).await
    }

    /// Calls the [`Leave`] action.
    pub async fn leave(
        &self,
        ctx: &mut Context<Ed>,
    ) -> Result<(), LeaveError> {
        Leave::from(self).call_inner(ctx).await
    }

    /// Calls the [`Pause`] action.
    pub async fn pause(
        &self,
        ctx: &mut Context<Ed>,
    ) -> Result<(), PauseError<Ed>> {
        Pause::from(self).call_inner(ctx).await
    }

    /// Calls the [`Resume`] action.
    pub async fn resume(
        &self,
        ctx: &mut Context<Ed>,
    ) -> Result<(), ResumeError<Ed>> {
        Resume::from(self).call_inner(ctx).await
    }

    /// Calls the [`Start`] action.
    pub async fn start(
        &self,
        ctx: &mut Context<Ed>,
    ) -> Result<SessionInfos<Ed>, StartError<Ed>> {
        let mut reporter =
            <Ed::ProgressReporter as ProgressReporter<_, Start<_>>>::new(ctx);
        Start::from(self).call_inner(&mut reporter, ctx).await
    }
}

impl<Ed: CollabEditor> Module<Ed> for Collab<Ed> {
    const NAME: &str = "collab";

    type Config = Config;

    fn api(&self, ctx: &mut ApiCtx<Ed>) {
        ctx.with_command(CopyId::from(self))
            .with_command(Join::from(self))
            .with_command(Jump::from(self))
            .with_command(Leave::from(self))
            .with_command(Pause::from(self))
            .with_command(Resume::from(self))
            .with_command(Start::from(self))
            .with_function(CopyId::from(self))
            .with_function(Jump::from(self))
            .with_function(Join::from(self))
            .with_function(Leave::from(self))
            .with_function(Resume::from(self))
            .with_function(Pause::from(self))
            .with_function(Start::from(self));
    }

    fn on_init(&self, ctx: &mut Context<Ed, Borrowed<'_>>) {
        Ed::on_init(ctx);
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
            auth_state: auth.state(),
            config: Default::default(),
            sessions: Default::default(),
            stop_channels: Default::default(),
        }
    }
}
