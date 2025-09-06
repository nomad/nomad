/// The type of error that can occur when the server's authenticator tries to
/// authenticate a peer from the provided
/// [`AccessToken`](auth_types::AccessToken).
#[derive(Debug, derive_more::Display, cauchy::Error, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[display("{_0}")]
pub enum AuthError {
    /// Authentication via GitHub failed.
    GitHub(GitHubAuthError),
}

/// The type of error that can occur when the server's authenticator tries to
/// authenticate a peer using a
/// [`GitHubAccessToken`](auth_types::GitHubAccessToken).
#[derive(Debug, derive_more::Display, cauchy::Error, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[display("{_0}")]
pub enum GitHubAuthError {
    /// GitHub's API returned an error.
    ApiError(String),

    /// Deserializing the response's body into JSON failed.
    DeserializeResponse(String),

    /// The HTTP request to GitHub's API failed.
    HttpRequest(String),
}
