use core::fmt;
use core::ops::Deref;
use std::rc::Rc;

use serde::de::{Deserialize, Deserializer};

#[derive(Clone)]
pub(crate) struct ServerSocket {
    inner: Rc<str>,
}

impl Default for ServerSocket {
    fn default() -> Self {
        Self { inner: "collab.nomad.foo:64420".to_owned().into() }
    }
}

impl fmt::Debug for ServerSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ServerSocket").field(&self.inner).finish()
    }
}

impl Deref for ServerSocket {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'de> Deserialize<'de> for ServerSocket {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner = String::deserialize(deserializer)?;
        Ok(Self { inner: inner.into() })
    }
}
