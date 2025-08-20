use auth_types::AuthInfos;
use editor::{Borrowed, Context};
use neovim::Neovim;
use neovim::notify::ContextExt;

use crate::{AuthEditor, login, logout};

impl AuthEditor for Neovim {
    type LoginError = core::convert::Infallible;

    #[allow(clippy::manual_async_fn)]
    fn credential_builder(
        _: &mut Context<Self, Borrowed>,
    ) -> impl Future<Output = Box<keyring::CredentialBuilder>> + Send + 'static
    {
        async move { keyring::default_credential_builder() }
    }

    async fn login(
        _: &mut Context<Self>,
    ) -> Result<AuthInfos, Self::LoginError> {
        Ok(AuthInfos { github_handle: "noib3".parse().expect("valid") })
    }

    fn on_login_error(
        error: login::LoginError<Self>,
        ctx: &mut Context<Self>,
    ) {
        ctx.notify_error(error);
    }

    fn on_logout_error(error: logout::LogoutError, ctx: &mut Context<Self>) {
        ctx.notify_error(error);
    }
}
