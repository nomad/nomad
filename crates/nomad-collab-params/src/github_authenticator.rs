use std::sync::LazyLock;

use auth_types::GitHubAccessToken;
use collab_types::GitHubHandle;
use http_client::HttpClient;

static GITHUB_USER_ENDPOINT_URL: LazyLock<http::Uri> =
    LazyLock::new(|| http::Uri::from_static("https://api.github.com/user"));

use crate::GitHubAuthError;

/// TODO: docs.
pub struct GitHubAuthenticator<T: HttpClient> {
    /// TODO: docs.
    pub http_client: T,
}

#[derive(serde::Deserialize)]
struct GitHubUserResponse {
    login: GitHubHandle,
}

#[derive(serde::Deserialize)]
struct GitHubUserResponseError {
    message: String,
}

impl<T: HttpClient> GitHubAuthenticator<T> {
    /// TODO: docs.
    pub async fn authenticate(
        &self,
        access_token: &GitHubAccessToken,
    ) -> Result<GitHubHandle, GitHubAuthError> {
        let request = http::Request::builder()
            .method(http::Method::GET)
            .uri((*GITHUB_USER_ENDPOINT_URL).clone())
            .header("Authorization", format!("token {access_token}"))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "Nomad")
            .body(String::new())
            .expect("all the fields are valid");

        let response =
            self.http_client.send(request).await.map_err(|err| {
                GitHubAuthError::HttpRequest(err.to_string())
            })?;

        if response.status().is_success() {
            let ok_response =
                serde_json::from_str::<GitHubUserResponse>(response.body())
                    .map_err(|err| {
                        GitHubAuthError::DeserializeResponse(err.to_string())
                    })?;

            Ok(ok_response.login)
        } else {
            let error_response =
                serde_json::from_str::<GitHubUserResponseError>(
                    response.body(),
                )
                .map_err(|err| {
                    GitHubAuthError::DeserializeResponse(err.to_string())
                })?;

            Err(GitHubAuthError::ApiError(error_response.message))
        }
    }
}
