use std::net::SocketAddr;
use std::path::PathBuf;

use collab_client::rustls_pki_types::{self, DnsName};
use collab_client::typestate::Optionals;
use collab_client::{rustls, Connector};
use nomad::prelude::WarningMsg;
use serde::Deserialize;
use url::Url;

/// TODO: docs
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct Config {
    /// TODO: docs
    #[serde(default = "default_enable")]
    enable: bool,

    /// TODO: docs
    #[serde(default = "default_project_dir")]
    project_dir: PathBuf,

    /// TODO: docs
    #[serde(default = "default_server_addr")]
    server_addr: Url,

    /// TODO: docs
    #[serde(default = "default_server_port")]
    server_port: u16,
}

fn default_enable() -> bool {
    true
}

fn default_project_dir() -> PathBuf {
    // TODO: this should be a path relative to the `/nomad` path.
    PathBuf::new()
}

fn default_server_addr() -> Url {
    Url::parse("tcp://collab.nomad.foo").expect("address is valid")
}

fn default_server_port() -> u16 {
    64420
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable: default_enable(),
            project_dir: default_project_dir(),
            server_addr: default_server_addr(),
            server_port: default_server_port(),
        }
    }
}

impl Config {
    pub(crate) fn connector(
        &self,
    ) -> Result<Connector<Optionals>, ConnectorError> {
        Ok(Connector::new()
            .server_addr(self.server_addr()?)
            .server_dns_name(self.server_dns_name()?)
            .unwrap()
            .tls_config(client_config()))
    }

    /// Returns the address of the server.
    fn server_addr(&self) -> Result<SocketAddr, ServerAddrError> {
        self.server_addr
            .socket_addrs(|| Some(self.server_port))?
            .into_iter()
            .next()
            .ok_or(ServerAddrError::EmptyAddresses)
    }

    fn server_dns_name(&self) -> Result<DnsName<'static>, DnsNameError> {
        self.server_addr
            .host_str()
            .ok_or(DnsNameError::AbsentHost)
            .and_then(|host| DnsName::try_from(host).map_err(Into::into))
            .map(|name| name.to_owned())
    }
}

/// Returns the TLS configuration used to connect to the server.
fn client_config() -> rustls::ClientConfig {
    let root_store = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };

    rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth()
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    #[error(transparent)]
    DnsName(#[from] DnsNameError),

    #[error(transparent)]
    ServerAddr(#[from] ServerAddrError),
}

/// The error type returned by [`CollabConfig::server_addr`].
#[derive(Debug, thiserror::Error)]
pub enum ServerAddrError {
    /// The URL resolved to an empty list of socket addresses.
    #[error("URL resolved to an empty list of socket addresses")]
    EmptyAddresses,

    /// The URL is invalid.
    #[error("{0}")]
    InvalidUrl(#[from] std::io::Error),
}

impl From<ServerAddrError> for WarningMsg {
    fn from(err: ServerAddrError) -> Self {
        let mut msg = WarningMsg::new();
        msg.add("couldn't resolve server address: ");
        msg.add(err.to_string().as_str());
        msg
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DnsNameError {
    #[error("host is absent")]
    AbsentHost,

    #[error(transparent)]
    InvalidHost(#[from] rustls_pki_types::InvalidDnsNameError),
}

#[cfg(test)]
mod tests {
    use super::*;

    impl PartialEq for ServerAddrError {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (Self::EmptyAddresses, Self::EmptyAddresses) => true,
                (Self::InvalidUrl(this), Self::InvalidUrl(other)) => {
                    this.kind() == other.kind()
                },
                _ => false,
            }
        }
    }

    impl PartialEq for DnsNameError {
        fn eq(&self, other: &Self) -> bool {
            matches!(
                (self, other),
                (Self::AbsentHost, Self::AbsentHost)
                    | (Self::InvalidHost(_), Self::InvalidHost(_))
            )
        }
    }

    /// Tests that the default `CollabConfig` can be created without panicking.
    #[test]
    fn collab_config_default() {
        let _config = Config::default();
    }

    /// Tests that the server address of the default `CollabConfig` can be
    /// resolved to a valid `SocketAddr`.
    #[test]
    fn collab_config_resolve_server_addr() {
        assert_eq!(Config::default().server_addr().map(|_| ()), Ok(()));
    }

    /// Tests that the server DNS name of the default `CollabConfig` can be
    /// resolved to a valid `DnsName`.
    #[test]
    fn collab_config_resolve_server_dns_name() {
        assert_eq!(Config::default().server_dns_name().map(|_| ()), Ok(()));
    }

    /// Tests that a session can be started using the connector created from
    /// the default `CollabConfig`.
    #[macro_rules_attribute::apply(smol_macros::test!)]
    async fn collab_config_connector_start() {
        let connector = Config::default().connector().unwrap();
        assert_eq!(connector.start().await.map(|_| ()), Ok(()));
    }
}
