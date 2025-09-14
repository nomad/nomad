use auth_types::AccessToken;
use collab_types::PeerHandle;
use editor::context::Borrowed;
use editor::{Access, Context};
use neovim::Neovim;
use neovim::notify::ContextExt;

use crate::{AuthEditor, config, github, login, logout};

impl AuthEditor for Neovim {
    type LoginError = github::GitHubLoginError<Self::HttpClient>;

    #[allow(clippy::manual_async_fn)]
    fn credential_builder(
        _: &mut Context<Self, Borrowed>,
    ) -> impl Future<Output = Box<keyring::CredentialBuilder>> + Send + 'static
    {
        async { keyring::default_credential_builder() }
    }

    async fn login(
        config: impl Access<config::Config>,
        ctx: &mut Context<Self>,
    ) -> Result<(AccessToken, PeerHandle), Self::LoginError> {
        let (access_token, github_handle) = github::login(config, ctx).await?;

        ctx.notify_info(format_args!(
            "Successfully logged in as '{github_handle}'",
        ));

        Ok((
            AccessToken::GitHub(access_token),
            PeerHandle::GitHub(github_handle),
        ))
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
