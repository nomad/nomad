//! TODO: docs.

use collab_server::message::GitHubHandle;
use ed::action::AsyncAction;
use ed::command::ToCompletionFn;
use ed::notify::{self, Name};
use ed::{AsyncCtx, Shared};

use crate::credential_store::CredentialStore;
use crate::{Auth, AuthBackend, AuthInfos};

/// TODO: docs.
#[derive(Clone, Default)]
pub struct Login {
    credential_store: CredentialStore,
    infos: Shared<Option<AuthInfos>>,
}

impl<B: AuthBackend> AsyncAction<B> for Login {
    const NAME: Name = "login";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), LoginError<B>> {
        if let Some(handle) = self.infos.with(|maybe_infos| {
            maybe_infos.as_ref().map(|infos| infos.handle().clone())
        }) {
            return Err(LoginError::AlreadyLoggedIn(handle));
        }

        let auth_infos = B::login(ctx).await.map_err(LoginError::Login)?;

        self.infos.set(Some(auth_infos.clone()));

        self.credential_store
            .get_entry()
            .await
            .map_err(LoginError::GetCredential)?
            .persist(auth_infos)
            .await
            .map_err(LoginError::PersistAuthInfos)
    }
}

/// TODO: docs.
pub enum LoginError<B: AuthBackend> {
    /// TODO: docs.
    AlreadyLoggedIn(GitHubHandle),

    /// TODO: docs.
    GetCredential(keyring::Error),

    /// TODO: docs.
    Login(B::LoginError),

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

impl<B: AuthBackend> ToCompletionFn<B> for Login {
    fn to_completion_fn(&self) {}
}

impl<B: AuthBackend> notify::Error for LoginError<B> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        todo!();
    }
}
