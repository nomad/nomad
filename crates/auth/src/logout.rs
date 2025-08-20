//! TODO: docs.

use auth_types::AuthInfos;
use editor::action::AsyncAction;
use editor::command::ToCompletionFn;
use editor::{Context, Editor, Shared};

use crate::credential_store::{self, CredentialStore};
use crate::{Auth, AuthEditor};

/// TODO: docs.
#[derive(Clone, Default)]
pub struct Logout {
    credential_store: CredentialStore,
    infos: Shared<Option<AuthInfos>>,
}

impl Logout {
    pub(crate) async fn call_inner<Ed: Editor>(
        &self,
        ctx: &mut Context<Ed>,
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

impl<Ed: AuthEditor> AsyncAction<Ed> for Logout {
    const NAME: &str = "logout";

    type Args = ();

    async fn call(&mut self, _: Self::Args, ctx: &mut Context<Ed>) {
        if let Err(err) = self.call_inner(ctx).await {
            Ed::on_logout_error(err, ctx);
        }
    }
}

/// TODO: docs.
#[derive(Debug, derive_more::Display, cauchy::Error)]
pub enum LogoutError {
    /// TODO: docs.
    #[display("Couldn't delete credentials from keyring: {_0}")]
    DeleteCredential(keyring::Error),

    /// TODO: docs.
    #[display("Couldn't get credentials from keyring: {_0}")]
    GetCredential(keyring::Error),

    /// TODO: docs.
    #[display("Not logged in")]
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

impl<Ed: Editor> ToCompletionFn<Ed> for Logout {
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
