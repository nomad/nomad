use ed::module::{ApiCtx, Module};
use ed::notify::Name;
use ed::{Borrowed, Context, Shared};

use crate::AuthEditor;
use crate::auth_infos::AuthInfos;
use crate::credential_store::CredentialStore;
use crate::login::Login;
use crate::logout::Logout;

/// TODO: docs.
#[derive(Default)]
pub struct Auth {
    pub(crate) credential_store: CredentialStore,
    pub(crate) infos: Shared<Option<AuthInfos>>,
}

impl Auth {
    /// Returns a shared handle to the `AuthInfos`, which will be `None` if the
    /// user hasn't logged in yet.
    pub fn infos(&self) -> &Shared<Option<AuthInfos>> {
        &self.infos
    }

    /// Returns a new instance of the [`Login`] action.
    pub fn login(&self) -> Login {
        self.into()
    }

    /// Returns a new instance of the [`Logout`] action.
    pub fn logout(&self) -> Logout {
        self.into()
    }

    /// TODO: docs.
    #[cfg(any(test, feature = "mock"))]
    #[track_caller]
    pub fn logged_in<Gh>(github_handle: Gh) -> Self
    where
        Gh: TryInto<collab_types::GitHubHandle>,
        Gh::Error: core::fmt::Debug,
    {
        Self {
            credential_store: CredentialStore::default(),
            infos: Shared::new(Some(AuthInfos::dummy(github_handle))),
        }
    }
}

impl<Ed: AuthEditor> Module<Ed> for Auth {
    const NAME: Name = "auth";

    type Config = ();

    fn api(&self, ctx: &mut ApiCtx<Ed>) {
        ctx.with_function(self.login()).with_function(self.logout());
    }

    fn on_init(&self, ctx: &mut Context<Ed, Borrowed>) {
        let credential_builder = Ed::credential_builder(ctx);
        let store = self.credential_store.clone();
        ctx.spawn_background(async move {
            store.set_builder(credential_builder.await);
        })
        .detach();

        let auth_infos = self.infos.clone();
        let store = self.credential_store.clone();
        ctx.spawn_and_detach(async move |ctx| {
            if let Some(infos) = ctx
                // Retrieving the credentials blocks, so do it in the
                // background.
                .spawn_background(async move { store.retrieve().await })
                .await
                .ok()
                .flatten()
            {
                auth_infos.set(Some(infos));
            }
        });
    }

    fn on_new_config(&self, _: Self::Config, _: &mut Context<Ed, Borrowed>) {}
}
