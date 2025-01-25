use collab_server::configs::nomad::NomadAuthenticateInfos;

/// TODO: docs.
#[derive(Debug, Clone)]
pub struct AuthInfos {
    inner: NomadAuthenticateInfos,
}

impl From<AuthInfos> for NomadAuthenticateInfos {
    fn from(auth_infos: AuthInfos) -> Self {
        auth_infos.inner
    }
}
