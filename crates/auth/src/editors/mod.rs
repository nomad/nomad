#[cfg(feature = "mock")]
pub mod mock;
#[cfg(feature = "neovim")]
mod neovim;

use core::fmt::Debug;

use auth_types::AuthInfos;
use editor::{Borrowed, Context, Editor};

use crate::{login, logout};

/// TODO: docs.
pub trait AuthEditor: Editor {
    /// TODO: docs.
    type LoginError: Debug;

    /// TODO: docs.
    fn credential_builder(
        ctx: &mut Context<Self, Borrowed>,
    ) -> impl Future<Output = Box<keyring::CredentialBuilder>> + Send + 'static;

    /// TODO: docs.
    fn login(
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = Result<AuthInfos, Self::LoginError>>;

    /// Called when the [`Login`](login::Login) action returns an error.
    fn on_login_error(error: login::LoginError<Self>, ctx: &mut Context<Self>);

    /// Called when the [`Logout`](logout::Logout) action returns an error.
    fn on_logout_error(error: logout::LogoutError, ctx: &mut Context<Self>);
}
