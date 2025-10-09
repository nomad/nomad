use editor::context::Borrowed;
use editor::module::{ApiCtx, Module};
use editor::{Context, Shared};

use crate::auth_state::AuthState;
use crate::credential_store::CredentialStore;
use crate::login::{Login, LoginError};
use crate::logout::{Logout, LogoutError};
use crate::{AuthEditor, Config};

/// TODO: docs.
#[derive(Default)]
pub struct Auth {
    pub(crate) config: Shared<Config>,
    pub(crate) credential_store: CredentialStore,
    pub(crate) state: AuthState,
}

impl Auth {
    /// Calls the [`Login`] action.
    pub async fn login<Ed: AuthEditor>(
        &self,
        ctx: &mut Context<Ed>,
    ) -> Result<(), LoginError<Ed>> {
        Login::from(self).call_inner(ctx).await
    }

    /// Calls the [`Logout`] action.
    pub async fn logout<Ed: AuthEditor>(
        &self,
        ctx: &mut Context<Ed>,
    ) -> Result<(), LogoutError> {
        Logout::from(self).call_inner(ctx).await
    }

    /// TODO: docs.
    #[cfg(feature = "mock")]
    #[track_caller]
    pub fn logged_in<Gh>(github_handle: Gh) -> Self
    where
        Gh: TryInto<peer_handle::GitHubHandle>,
        Gh::Error: core::fmt::Debug,
    {
        let github_handle =
            github_handle.try_into().expect("invalid GitHub handle");

        let this = Self::default();

        this.state.set_logged_in(auth_types::JsonWebToken::mock(
            peer_handle::PeerHandle::GitHub(github_handle),
        ));

        this
    }

    /// Returns a handle to the [`AuthState`].
    pub fn state(&self) -> AuthState {
        self.state.clone()
    }
}

impl<Ed: AuthEditor> Module<Ed> for Auth {
    const NAME: &str = "auth";

    type Config = Config;

    fn api(&self, ctx: &mut ApiCtx<Ed>) {
        ctx.with_function(Login::from(self)).with_function(Logout::from(self));
    }

    fn on_init(&self, ctx: &mut Context<Ed, Borrowed>) {
        let credential_builder = Ed::credential_builder(ctx);
        let store = self.credential_store.clone();
        ctx.spawn_background(async move {
            store.set_builder(credential_builder.await);
        })
        .detach();

        let auth_state = self.state();
        let store = self.credential_store.clone();
        ctx.spawn_and_detach(async move |ctx| {
            // Retrieving the credentials blocks, so do it in the background.
            match ctx
                .spawn_background(async move { store.retrieve().await })
                .await
            {
                Ok(Some(jwt)) => auth_state.set_logged_in(jwt),
                Ok(None) => {},
                Err(err) => {
                    tracing::error!("couldn't retrieve credentials: {err}")
                },
            }
        });
    }

    fn on_new_config(&self, _: Self::Config, _: &mut Context<Ed, Borrowed>) {}
}
