//! This crate contains the [`Params`][NomadParams] used by Nomad's collab
//! server running at `collab.nomad.foo`.

mod auth_error;
#[cfg(feature = "github-authenticator")]
mod github_authenticator;
mod nomad_params;

pub use auth_error::{AuthError, GitHubAuthError};
#[cfg(feature = "github-authenticator")]
pub use github_authenticator::GitHubAuthenticator;
pub use nomad_params::NomadParams;
pub use ulid;
