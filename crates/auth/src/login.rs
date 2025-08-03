//! TODO: docs.

use collab_types::GitHubHandle;
use ed::action::AsyncAction;
use ed::command::ToCompletionFn;
use ed::notify::{self, Name};
use ed::{Context, Shared};

use crate::credential_store::{self, CredentialStore};
use crate::{Auth, AuthEditor, AuthInfos};

/// TODO: docs.
#[derive(Clone, Default)]
pub struct Login {
    credential_store: CredentialStore,
    infos: Shared<Option<AuthInfos>>,
}

impl<Ed: AuthEditor> AsyncAction<Ed> for Login {
    const NAME: Name = "login";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut Context<Ed>,
    ) -> Result<(), LoginError<Ed>> {
        if let Some(handle) = self.infos.with(|maybe_infos| {
            maybe_infos.as_ref().map(|infos| infos.handle().clone())
        }) {
            return Err(LoginError::AlreadyLoggedIn(handle));
        }

        let auth_infos = Ed::login(ctx).await.map_err(LoginError::Login)?;

        self.infos.set(Some(auth_infos.clone()));

        // Persisting the credentials blocks, so do it in the background.
        let credential_store = self.credential_store.clone();
        ctx.spawn_background(async move {
            credential_store.persist(auth_infos).await
        })
        .await
        .map_err(Into::into)
    }
}

/// TODO: docs.
pub enum LoginError<Ed: AuthEditor> {
    /// TODO: docs.
    AlreadyLoggedIn(GitHubHandle),

    /// TODO: docs.
    GetCredential(keyring::Error),

    /// TODO: docs.
    Login(Ed::LoginError),

    /// TODO: docs.
    PersistAuthInfos(keyring::Error),
}

impl From<&Auth> for Login {
    fn from(auth: &Auth) -> Self {
        Self {
            credential_store: auth.credential_store.clone(),
            infos: auth.infos().clone(),
        }
    }
}

impl<Ed: AuthEditor> ToCompletionFn<Ed> for Login {
    fn to_completion_fn(&self) {}
}

impl<Ed: AuthEditor> From<credential_store::Error> for LoginError<Ed> {
    fn from(err: credential_store::Error) -> Self {
        use credential_store::Error::*;
        match err {
            GetCredential(err) => Self::GetCredential(err),
            Op(err) => Self::PersistAuthInfos(err),
        }
    }
}

impl<Ed: AuthEditor> notify::Error for LoginError<Ed> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        let mut msg = notify::Message::new();
        match self {
            Self::AlreadyLoggedIn(handle) => {
                msg.push_str("Already logged in as ")
                    .push_info(handle.as_str());
            },
            Self::GetCredential(err) => {
                msg.push_str("Couldn't get credential from keyring: ")
                    .push_str(err.to_string());
            },
            Self::Login(err) => return err.to_message(),
            Self::PersistAuthInfos(err) => {
                msg.push_str("Couldn't persist credentials: ")
                    .push_str(err.to_string());
            },
        }
        (notify::Level::Error, msg)
    }
}
