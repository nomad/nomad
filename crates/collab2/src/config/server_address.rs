use core::fmt;
use core::ops::Deref;
use std::rc::Rc;

use serde::de::{Deserialize, Deserializer};

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
