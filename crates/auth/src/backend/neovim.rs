use collab_server::message::GitHubHandle;
use collab_server::nomad::NomadAuthenticateInfos;
use ed::{AsyncCtx, EditorCtx};
use neovim::Neovim;

use crate::AuthInfos;
use crate::backend::AuthBackend;

impl AuthBackend for Neovim {
    type LoginError = core::convert::Infallible;

    #[allow(clippy::manual_async_fn)]
    fn credential_builder(
        _: &mut EditorCtx<Self>,
    ) -> impl Future<Output = Box<keyring::CredentialBuilder>> + Send + 'static
    {
        async move { keyring::default_credential_builder() }
    }

    async fn login(
        _: &mut AsyncCtx<'_, Self>,
    ) -> Result<AuthInfos, Self::LoginError> {
        Ok(NomadAuthenticateInfos {
            github_handle: "noib3".parse::<GitHubHandle>().expect("valid"),
        }
        .into())
    }
}
