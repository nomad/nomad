use std::path::PathBuf;

use serde::Deserialize;
use url::Url;

/// TODO: docs
#[derive(Debug, Deserialize)]
pub struct CollabConfig {
    /// TODO: docs
    project_dir: PathBuf,

    /// TODO: docs
    server_address: Url,

    /// TODO: docs
    server_port: u16,
}

impl Default for CollabConfig {
    #[inline]
    fn default() -> Self {
        Self {
            // TODO: this should be a path relative to the `/nomad` path.
            project_dir: PathBuf::new(),
            server_address: Url::parse("tcp://collab.nomad.foo").unwrap(),
            server_port: 64420,
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
            .server_address
            .socket_addrs(|| Some(config.server_port))
            .unwrap();

        assert_eq!(addrs.len(), 1);
    }
}
