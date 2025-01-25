use core::marker::PhantomData;

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

/// The type of error that can occur when [`Start`]ing a new session fails.
pub enum StartError<B: CollabBackend> {
    NoBufferFocused(NoBufferFocusedError<B>),
    SearchProjectRoot(B::SearchProjectRootError),
    UserNotLoggedIn(UserNotLoggedInError<B>),
}

pub struct NoBufferFocusedError<B>(PhantomData<B>);
pub struct UserNotLoggedInError<B>(PhantomData<B>);

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
            .ok_or_else(StartError::user_not_logged_in)?;

        let buffer_id = ctx.with_ctx(|ctx| {
            ctx.current_buffer()
                .map(|buf| buf.id())
                .ok_or_else(StartError::no_buffer_focused)
        })?;

        let _project_root = B::search_project_root(buffer_id, ctx)
            .await
            .map_err(StartError::SearchProjectRoot)?;

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

impl<B: CollabBackend> StartError<B> {
    fn no_buffer_focused() -> Self {
        Self::NoBufferFocused(NoBufferFocusedError(PhantomData))
    }

    fn user_not_logged_in() -> Self {
        Self::UserNotLoggedIn(UserNotLoggedInError(PhantomData))
    }
}

impl<B: CollabBackend> notify::Error for StartError<B> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            StartError::NoBufferFocused(err) => err.to_message(),
            StartError::SearchProjectRoot(err) => err.to_message(),
            StartError::UserNotLoggedIn(err) => err.to_message(),
        }
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
