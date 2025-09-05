//! TODO: docs.

use std::io;
use std::sync::LazyLock;

use auth_types::{GitHubAccessToken, GitHubHandle, OAuthState};
use editor::{Access, Context, Editor};
use futures_util::{FutureExt, future, pin_mut};
use rand::Rng;
use url::Url;

use crate::Config;

static GITHUB_AUTHORIZE_URL: LazyLock<Url> = LazyLock::new(|| {
    Url::parse("https://github.com/login/oauth/authorize").expect("valid URL")
});

pub(crate) async fn login<Ed: Editor>(
    config: impl Access<Config>,
    ctx: &mut Context<Ed>,
) -> Result<(GitHubAccessToken, GitHubHandle), GitHubLoginError> {
    let auth_server_url = config.with(|config| config.server_url.clone());

    let oauth_state = OAuthState::from_bytes(ctx.with_rng(Rng::random));

    let login_request = ctx.spawn_background({
        let auth_server_url = auth_server_url.clone();
        async move { login_request(&auth_server_url, &oauth_state).await }
    });

    let open_browser = ctx
        .spawn_background(async move {
            open_browser(&auth_server_url, &oauth_state)
        })
        .fuse();

    pin_mut!(login_request);
    pin_mut!(open_browser);

    let login_result = loop {
        match future::select(&mut login_request, &mut open_browser).await {
            future::Either::Left((login_result, _)) => break login_result,
            future::Either::Right((open_result, _)) => {
                open_result.map_err(GitHubLoginError::OpenBrowser)?;
            },
        }
    };

    let _access_token = login_result.map_err(GitHubLoginError::LoginRequest);

    todo!()
}

async fn login_request(
    auth_server_url: &Url,
    oauth_state: &OAuthState,
) -> reqwest::Result<GitHubAccessToken> {
    let login_url = auth_server_url
        .join(&format!("/github/login/{oauth_state}"))
        .expect("route is valid");

    reqwest::get(login_url).await?.json::<GitHubAccessToken>().await
}

fn open_browser(
    auth_server_url: &Url,
    oauth_state: &OAuthState,
) -> io::Result<()> {
    let callback_url =
        auth_server_url.join("/github/callback").expect("route is valid");

    let mut github_authorize_url = (*GITHUB_AUTHORIZE_URL).clone();

    github_authorize_url
        .query_pairs_mut()
        .append_pair("client_id", auth_types::NOMAD_GITHUB_CLIENT_ID.as_str())
        .append_pair("scope", "read:user user:email")
        .append_pair("state", &oauth_state.to_string())
        .append_pair("redirect_uri", callback_url.as_str());

    open::that(github_authorize_url.as_str())
}

/// TODO: docs.
#[derive(Debug, derive_more::Display)]
#[display("{_0}")]
pub enum GitHubLoginError {
    /// The login request to the authentication server failed.
    #[display("Login request to the authentication server failed: {_0}")]
    LoginRequest(reqwest::Error),

    /// The user's web browser couldn't be opened.
    #[display("Couldn't open URL in web browser: {_0}")]
    OpenBrowser(io::Error),
}
