/// The type of error that can occur when the server's authenticator tries to
/// authenticate a peer from the provided JWT.
#[derive(Debug, derive_more::Display, cauchy::Error, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AuthError {
    /// The JWT was invalid.
    #[display("{_0}")]
    Jwt(String),

    /// The client is out of date.
    #[display(
        "your Nomad version is out of date. Please update Nomad to the \
         latest version to continue using the collaboration features"
    )]
    OutdatedClient,
}
