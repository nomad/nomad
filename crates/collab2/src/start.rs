use auth::AuthInfos;
use nvimx2::action::AsyncAction;
use nvimx2::command::ToCompletionFn;
use nvimx2::notify::{self, Name};
use nvimx2::{AsyncCtx, Shared};

use crate::config::Config;
use crate::{Collab, CollabBackend};

/// The [`Action`] used to start a new collaborative editing session.
#[derive(Clone)]
pub struct Start {
    auth_infos: Shared<Option<AuthInfos>>,
    _config: Shared<Config>,
}

pub enum StartError<B: CollabBackend> {
    InvalidBufferPath(String),
    NoBufferFocused,
    UserNotLoggedIn,
    FindProjectRoot(B::FindProjectRootError),
}

impl<B: CollabBackend> AsyncAction<B> for Start {
    const NAME: Name = "start";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), StartError<B>> {
        let _auth_infos = self
            .auth_infos
            .with(|infos| infos.as_ref().cloned())
            .ok_or(StartError::UserNotLoggedIn)?;

        let buffer_id = ctx.with_ctx(|ctx| {
            ctx.current_buffer()
                .map(|buf| buf.id())
                .ok_or(StartError::NoBufferFocused)
        })?;

        let _project_root = B::project_root(buffer_id, ctx)
            .await
            .map_err(StartError::FindProjectRoot)?;

        Ok(())
    }
}

impl<B: CollabBackend> ToCompletionFn<B> for Start {
    fn to_completion_fn(&self) {}
}

impl From<&Collab> for Start {
    fn from(collab: &Collab) -> Self {
        Self {
            auth_infos: collab.auth_infos.clone(),
            _config: collab.config.clone(),
        }
    }
}

impl<B: CollabBackend> notify::Error for StartError<B> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            StartError::InvalidBufferPath(_path) => todo!(),
            StartError::NoBufferFocused => todo!(),
            StartError::UserNotLoggedIn => todo!(),
            StartError::FindProjectRoot(err) => err.to_message(),
        }
    }
}
