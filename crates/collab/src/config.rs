use std::path::PathBuf;

use serde::Deserialize;
use url::Url;

/// TODO: docs
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollabConfig {
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

#[inline]
fn default_enable() -> bool {
    true
}

#[inline]
fn default_project_dir() -> PathBuf {
    // TODO: this should be a path relative to the `/nomad` path.
    PathBuf::new()
}

#[inline]
fn default_server_addr() -> Url {
    Url::parse("tcp://collab.nomad.foo").unwrap()
}

#[inline]
fn default_server_port() -> u16 {
    64420
}

impl Default for CollabConfig {
    #[inline]
    fn default() -> Self {
        Self {
            enable: default_enable(),
            project_dir: default_project_dir(),
            server_addr: default_server_addr(),
            server_port: default_server_port(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that the default `CollabConfig` can be created without panicking.
    #[test]
    fn collab_config_default() {
        let _config = CollabConfig::default();
    }

    /// Tests that the server address of the default `CollabConfig` can be
    /// resolved to a valid `SocketAddr`.
    #[test]
    fn collab_config_resolve_server_addr() {
        let config = CollabConfig::default();

        let addrs = config
            .server_addr
            .socket_addrs(|| Some(config.server_port))
            .unwrap();

        assert_eq!(addrs.len(), 1);
    }
}
