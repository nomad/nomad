#[cfg(feature = "mock")]
pub mod mock;
#[cfg(feature = "neovim")]
mod neovim;

use ed::EditorCtx;
use ed::backend::Backend;

/// TODO: docs.
pub trait AuthBackend: Backend {
    /// TODO: docs.
    fn credential_store(
        ctx: &mut EditorCtx<Self>,
    ) -> impl Future<Output = Box<keyring::CredentialBuilder>> + Send + 'static;
}
