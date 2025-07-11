//! TODO: docs.

use core::fmt;
use core::ops::Deref;
use std::rc::Rc;

use abs_path::AbsPathBuf;
use serde::de::{Deserialize, Deserializer};

/// TODO: docs.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The address of the server to connect to when starting or joining an
    /// editing session.
    pub(crate) server_address: ServerAddress,

    /// TODO: docs.
    pub(crate) store_remote_projects_under: Option<AbsPathBuf>,
}

/// TODO: docs.
#[derive(Clone)]
pub struct ServerAddress {
    inner: Rc<str>,
}

impl Default for ServerAddress {
    fn default() -> Self {
        Self { inner: "collab.nomad.foo:64420".to_owned().into() }
    }
}

impl fmt::Debug for ServerAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ServerAddress").field(&self.inner).finish()
    }
}

impl Deref for ServerAddress {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'de> Deserialize<'de> for ServerAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner = String::deserialize(deserializer)?;
        Ok(Self { inner: inner.into() })
    }
}
