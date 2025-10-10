//! TODO: docs.

mod audience;
mod claims;
mod email_address;
mod github_client_id;
mod json_web_token;
mod oauth_state;

pub use audience::{Audience, Client, CollabServer};
pub use claims::{Claims, Subject};
pub use email_address::EmailAddress;
pub use github_client_id::GitHubClientId;
pub use json_web_token::JsonWebToken;
pub use jsonwebtoken;
pub use oauth_state::{OAuthState, OAuthStateFromStrError};
pub use peer_handle::{GitHubHandle, PeerHandle};

/// TODO: docs.
pub const API_VERSION: u32 = 1;

/// The [`issuer`](Claims::issuer) to expect in JWTs issued by Nomad's auth
/// server.
pub const JWT_ISSUER: &str = "auth.nomad.foo";

/// The [`GitHubClientId`] assigned to Nomad's OAuth app.
pub const NOMAD_GITHUB_CLIENT_ID: GitHubClientId =
    GitHubClientId("Ov23liDqf1zOSVXAoVJq");
