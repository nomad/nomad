//! TODO: docs.

use core::pin::pin;
use core::str::FromStr;
use std::io;
use std::sync::LazyLock;

use auth_types::{JsonWebToken, OAuthState};
use editor::{Access, Context, Editor};
use futures_util::{FutureExt, future, pin_mut};
use http_client::HttpClient;
use rand::Rng;
use url::Url;

use crate::Config;

static GITHUB_AUTHORIZE_URL: LazyLock<Url> = LazyLock::new(|| {
    Url::parse("https://github.com/login/oauth/authorize").expect("valid URL")
});

pub(crate) async fn login<Ed: Editor>(
    config: impl Access<Config>,
    ctx: &mut Context<Ed>,
) -> Result<JsonWebToken, GitHubLoginError<Ed::HttpClient>> {
    let auth_server_url = config.with(|config| config.server_url.clone());
    let oauth_state = OAuthState::from_bytes(ctx.with_rng(Rng::random));
    let http_client = ctx.http_client();

    let mut login_request =
        pin!(login_request(&http_client, &auth_server_url, &oauth_state));

    let open_browser = ctx
        .spawn_background({
            let auth_server_url = auth_server_url.clone();
            async move { open_browser(&auth_server_url, &oauth_state) }
        })
        .fuse();

    pin_mut!(open_browser);

    loop {
        match future::select(&mut login_request, &mut open_browser).await {
            future::Either::Left((login_result, _)) => break login_result,
            future::Either::Right((open_result, _)) => {
                open_result.map_err(GitHubLoginError::OpenBrowser)?;
            },
        }
    }
}

async fn login_request<Client: HttpClient>(
    http_client: &Client,
    auth_server_url: &Url,
    oauth_state: &OAuthState,
) -> Result<JsonWebToken, GitHubLoginError<Client>> {
    let login_url = auth_server_url
        .join(&format!("/github/login/{oauth_state}"))
        .expect("route is valid");

    let request = http::Request::builder()
        .method(http::Method::GET)
        .uri(login_url.to_string())
        .body(String::new())
        .expect("all the fields are valid");

    let response = http_client
        .send(request)
        .await
        .map_err(GitHubLoginError::LoginRequest)?;

    response.body().parse().map_err(GitHubLoginError::ParseResponse)
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
#[derive(cauchy::Debug, derive_more::Display)]
#[display("{_0}")]
pub enum GitHubLoginError<Client: HttpClient> {
    /// The login request to the authentication server failed.
    #[display("Login request to the authentication server failed: {_0}")]
    LoginRequest(Client::Error),

    /// The user's web browser couldn't be opened.
    #[display("Couldn't open URL in web browser: {_0}")]
    OpenBrowser(io::Error),

    /// The body of the response we got from the auth server couldn't be parsed
    /// into a JWT.
    #[display("Couldn't deserialize response into authentication token: {_0}")]
    ParseResponse(<JsonWebToken as FromStr>::Err),
}
