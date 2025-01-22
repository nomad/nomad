use auth::AuthInfos;
use nvimx2::action::AsyncAction;
use nvimx2::backend::Backend;
use nvimx2::command::ToCompletionFn;
use nvimx2::notify::{self, Name};
use nvimx2::{AsyncCtx, Shared, fs};

use crate::Collab;
use crate::config::Config;

/// The [`Action`] used to start a new collaborative editing session.
#[derive(Clone)]
pub struct Start {
    auth_infos: Shared<Option<AuthInfos>>,
    _config: Shared<Config>,
}

pub enum StartError {
    InvalidBufferPath(String),
    NoBufferFocused,
    UserNotLoggedIn,
}

impl<B: Backend> AsyncAction<B> for Start {
    const NAME: Name = "start";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), StartError> {
        let _auth_infos = self
            .auth_infos
            .with(|infos| infos.as_ref().cloned())
            .ok_or(StartError::UserNotLoggedIn)?;

        let _buffer_path: fs::AbsPathBuf = ctx.with_ctx(|ctx| {
            ctx.current_buffer().ok_or(StartError::NoBufferFocused).and_then(
                |buffer| {
                    buffer.name().parse().map_err(|_| {
                        StartError::InvalidBufferPath(
                            buffer.name().into_owned(),
                        )
                    })
                },
            )
        })?;

        Ok(())
    }
}

impl<B: Backend> ToCompletionFn<B> for Start {
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

impl notify::Error for StartError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        todo!();
    }
}
