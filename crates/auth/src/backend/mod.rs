#[cfg(feature = "mock")]
pub mod mock;
#[cfg(feature = "neovim")]
mod neovim;

use core::fmt::Debug;

use ed::backend::Backend;
use ed::{Borrowed, Context, notify};

use crate::AuthInfos;

/// TODO: docs.
pub trait AuthBackend: Backend {
    /// TODO: docs.
    type LoginError: Debug + notify::Error;

    /// TODO: docs.
    fn credential_builder(
        ctx: &mut Context<Self, Borrowed>,
    ) -> impl Future<Output = Box<keyring::CredentialBuilder>> + Send + 'static;

    /// TODO: docs.
    fn login(
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = Result<AuthInfos, Self::LoginError>>;
}
