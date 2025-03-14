//! TODO: docs.

use ed::action::AsyncAction;
use ed::backend::Backend;
use ed::command::ToCompletionFn;
use ed::notify::{self, Name};
use ed::{AsyncCtx, Shared};

use crate::credential_store::CredentialStore;
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
        _: &mut AsyncCtx<'_, B>,
    ) -> Result<(), LogoutError> {
        self.infos.with_mut(|maybe_infos| {
            if maybe_infos.is_some() {
                *maybe_infos = None;
                Ok(())
            } else {
                Err(LogoutError::NotLoggedIn)
            }
        })?;

        self.credential_store
            .get_entry()
            .await
            .map_err(LogoutError::GetCredential)?
            .delete()
            .await
            .map_err(LogoutError::DeleteCredential)
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

impl notify::Error for LogoutError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        todo!();
    }
}
