use collab_server::configs::nomad::NomadAuthenticateInfos;

/// TODO: docs.
#[derive(Debug, Clone)]
pub struct AuthInfos {
    inner: NomadAuthenticateInfos,
}

impl AuthInfos {
    #[cfg(any(test, feature = "test"))]
    #[track_caller]
    pub(crate) fn dummy<Gh>(github_handle: Gh) -> Self
    where
        Gh: TryInto<collab_server::message::GitHubHandle>,
        Gh::Error: core::fmt::Debug,
    {
        Self {
            inner: NomadAuthenticateInfos {
                github_handle: github_handle
                    .try_into()
                    .expect("invalid github handle"),
            },
        }
    }
}

impl From<AuthInfos> for NomadAuthenticateInfos {
    fn from(auth_infos: AuthInfos) -> Self {
        auth_infos.inner
    }
}
