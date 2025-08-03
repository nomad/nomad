use collab_server::nomad::NomadAuthenticateInfos;
use collab_types::GitHubHandle;

/// TODO: docs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct AuthInfos {
    inner: NomadAuthenticateInfos,
}

impl AuthInfos {
    /// TODO: docs.
    pub fn handle(&self) -> &GitHubHandle {
        &self.inner.github_handle
    }

    #[cfg(any(test, feature = "mock"))]
    #[track_caller]
    pub(crate) fn dummy<Gh>(github_handle: Gh) -> Self
    where
        Gh: TryInto<GitHubHandle, Error: core::fmt::Debug>,
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

impl AsRef<GitHubHandle> for AuthInfos {
    fn as_ref(&self) -> &GitHubHandle {
        self.handle()
    }
}

impl From<NomadAuthenticateInfos> for AuthInfos {
    fn from(inner: NomadAuthenticateInfos) -> Self {
        Self { inner }
    }
}

impl From<AuthInfos> for NomadAuthenticateInfos {
    fn from(infos: AuthInfos) -> Self {
        infos.inner
    }
}
