use nvimx2::backend::Backend;
use nvimx2::module::{ApiCtx, Module};
use nvimx2::notify::Name;
use nvimx2::{EditorCtx, Shared};

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

impl<B: Backend> Module<B> for Auth {
    const NAME: Name = "auth";

    type Config = ();

    fn api(&self, ctx: &mut ApiCtx<B>) {
        ctx.with_function(self.login()).with_function(self.logout());
    }

    fn on_init(&self, _: &mut EditorCtx<B>) {}

    fn on_new_config(&self, _: Self::Config, _: &mut EditorCtx<B>) {}
}
