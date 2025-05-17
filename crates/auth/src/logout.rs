//! TODO: docs.

use ed::action::AsyncAction;
use ed::backend::Backend;
use ed::command::ToCompletionFn;
use ed::notify::{self, Name};
use ed::{Context, Shared};

use crate::credential_store::{self, CredentialStore};
use crate::{Auth, AuthInfos};

/// TODO: docs.
#[derive(Clone, Default)]
pub struct Logout {
    credential_store: CredentialStore,
    infos: Shared<Option<AuthInfos>>,
}

impl<B: Backend> AsyncAction<B> for Logout {
    const NAME: Name = "logout";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut Context<B>,
    ) -> Result<(), LogoutError> {
        self.infos.with_mut(|maybe_infos| {
            if maybe_infos.is_some() {
                *maybe_infos = None;
                Ok(())
            } else {
                Err(LogoutError::NotLoggedIn)
            }
        })?;

        // Deleting the credentials blocks, so do it in the background.
        let credential_store = self.credential_store.clone();
        ctx.spawn_background(async move { credential_store.delete().await })
            .await
            .map_err(Into::into)
    }
}

/// TODO: docs.
pub enum LogoutError {
    /// TODO: docs.
    DeleteCredential(keyring::Error),

    /// TODO: docs.
    GetCredential(keyring::Error),

    /// TODO: docs.
    NotLoggedIn,
}

impl From<&Auth> for Logout {
    fn from(auth: &Auth) -> Self {
        Self {
            credential_store: auth.credential_store.clone(),
            infos: auth.infos().clone(),
        }
    }
}

impl<B: Backend> ToCompletionFn<B> for Logout {
    fn to_completion_fn(&self) {}
}

impl From<credential_store::Error> for LogoutError {
    fn from(err: credential_store::Error) -> Self {
        use credential_store::Error::*;
        match err {
            GetCredential(err) => Self::GetCredential(err),
            Op(err) => Self::DeleteCredential(err),
        }
    }
}

impl notify::Error for LogoutError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        let mut msg = notify::Message::new();
        match self {
            Self::DeleteCredential(err) => {
                msg.push_str("couldn't delete credentials from keyring: ")
                    .push_str(err.to_string());
            },
            Self::GetCredential(err) => {
                msg.push_str("couldn't get credential from keyring: ")
                    .push_str(err.to_string());
            },
            Self::NotLoggedIn => {
                msg.push_str("not logged in");
            },
        }
        (notify::Level::Error, msg)
    }
}
