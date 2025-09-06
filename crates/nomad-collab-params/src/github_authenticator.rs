use std::sync::LazyLock;

use auth_types::GitHubAccessToken;
use collab_types::GitHubHandle;
use url::Url;

const GITHUB_USER_ENDPOINT_URL: LazyLock<Url> = LazyLock::new(|| {
    Url::parse("https://api.github.com/user").expect("valid URL")
});

use crate::GitHubAuthError;

/// TODO: docs.
pub struct GitHubAuthenticator<'http_client> {
    http_client: &'http_client reqwest::Client,
}

#[derive(serde::Deserialize)]
struct GitHubUserResponse {
    login: GitHubHandle,
}

#[derive(serde::Deserialize)]
struct GitHubUserResponseError {
    message: String,
}

impl<'http_client> GitHubAuthenticator<'http_client> {
    /// TODO: docs.
    pub async fn authenticate(
        &self,
        access_token: &GitHubAccessToken,
    ) -> Result<GitHubHandle, GitHubAuthError> {
        let response = self
            .http_client
            .get((&*GITHUB_USER_ENDPOINT_URL).clone())
            .header("Authorization", format!("token {access_token}"))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "Nomad")
            .send()
            .await
            .map_err(|err| GitHubAuthError::HttpRequest(err.to_string()))?;

        if response.status().is_success() {
            let ok_response = response
                .json::<GitHubUserResponse>()
                .await
                .map_err(|err| {
                    GitHubAuthError::DeserializeResponse(err.to_string())
                })?;

            Ok(ok_response.login)
        } else {
            let error_response = response
                .json::<GitHubUserResponseError>()
                .await
                .map_err(|err| {
                    GitHubAuthError::DeserializeResponse(err.to_string())
                })?;

            Err(GitHubAuthError::ApiError(error_response.message))
        }
    }

    /// Creates a new [`GitHubAuthenticator`] with the given HTTP client.
    pub fn new(http_client: &'http_client reqwest::Client) -> Self {
        Self { http_client }
    }
}
