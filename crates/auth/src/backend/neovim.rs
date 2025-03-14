use ed::neovim::Neovim;

use crate::backend::AuthBackend;

impl AuthBackend for Neovim {
    #[allow(clippy::manual_async_fn)]
    fn credential_store(
        _: &mut ed::EditorCtx<Self>,
    ) -> impl Future<Output = Box<keyring::CredentialBuilder>> + Send + 'static
    {
        async move { keyring::builtin_credential_builder() }
    }
}
