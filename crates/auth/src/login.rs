//! TODO: docs.

use auth_types::{AuthInfos, GitHubHandle};
use editor::action::AsyncAction;
use editor::command::ToCompletionFn;
use editor::{Context, Shared};

use crate::credential_store::{self, CredentialStore};
use crate::{Auth, AuthEditor};

/// TODO: docs.
#[derive(Clone, Default)]
pub struct Login {
    credential_store: CredentialStore,
    infos: Shared<Option<AuthInfos>>,
}

impl Login {
    pub(crate) async fn call_inner<Ed: AuthEditor>(
        &self,
        ctx: &mut Context<Ed>,
    ) -> Result<(), LoginError<Ed>> {
        if let Some(handle) = self.infos.with(|maybe_infos| {
            maybe_infos.as_ref().map(|infos| infos.github_handle.clone())
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

impl<Ed: AuthEditor> AsyncAction<Ed> for Login {
    const NAME: &str = "login";

    type Args = ();

    async fn call(&mut self, _: Self::Args, ctx: &mut Context<Ed>) {
        if let Err(err) = self.call_inner(ctx).await {
            Ed::on_login_error(err, ctx);
        }
    }
}

/// TODO: docs.
#[derive(cauchy::Debug, derive_more::Display, cauchy::Error)]
pub enum LoginError<Ed: AuthEditor> {
    /// TODO: docs.
    #[display("Already logged in as {_0}")]
    AlreadyLoggedIn(GitHubHandle),

    /// TODO: docs.
    #[display("Couldn't get credentials from keyring: {_0}")]
    GetCredential(keyring::Error),

    /// TODO: docs.
    #[display("{_0}")]
    Login(Ed::LoginError),

    /// TODO: docs.
    #[display("Couldn't persist credentials: {_0}")]
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
