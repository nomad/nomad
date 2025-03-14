use ed::module::{ApiCtx, Module};
use ed::notify::Name;
use ed::{EditorCtx, Shared};

use crate::AuthBackend;
use crate::auth_infos::AuthInfos;
use crate::login::Login;
use crate::logout::Logout;

/// TODO: docs.
#[derive(Default)]
pub struct Auth {
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
    pub fn dummy<Gh>(github_handle: Gh) -> Self
    where
        Gh: TryInto<collab_server::message::GitHubHandle>,
        Gh::Error: core::fmt::Debug,
    {
        Self { infos: Shared::new(Some(AuthInfos::dummy(github_handle))) }
    }
}

impl<B: AuthBackend> Module<B> for Auth {
    const NAME: Name = "auth";

    type Config = ();

    fn api(&self, ctx: &mut ApiCtx<B>) {
        ctx.with_function(self.login()).with_function(self.logout());
    }

    fn on_init(&self, ctx: &mut EditorCtx<B>) {
        let fut = B::credential_store(ctx);

        ctx.spawn_background(async move {
            let _store = fut.await;
        })
        .detach();
    }

    fn on_new_config(&self, _: Self::Config, _: &mut EditorCtx<B>) {}
}
