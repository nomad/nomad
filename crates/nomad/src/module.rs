//! TODO: docs

use std::fmt;
use std::future::Future;
use std::hash::{Hash, Hasher};

use serde::de::DeserializeOwned;

use crate::{Api, Get, MaybeResult};

/// TODO: docs
pub trait Module: 'static + Sized {
    /// TODO: docs
    const NAME: ModuleName;

    /// TODO: docs
    type Config: Default + DeserializeOwned;

    /// TODO: docs
    fn init(config: Get<Self::Config>) -> Api<Self>;

    /// TODO: docs
    fn run(&self) -> impl Future<Output = impl MaybeResult<()>>;
}

/// TODO: docs
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleName {
    name: &'static str,
}

impl fmt::Debug for ModuleName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("ModuleName").field(&self.name).finish()
    }
}

impl fmt::Display for ModuleName {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name)
    }
}

impl AsRef<str> for ModuleName {
    #[inline]
    fn as_ref(&self) -> &str {
        self.name
    }
}

impl ModuleName {
    /// TODO: docs
    #[inline]
    pub(crate) fn as_str(&self) -> &'static str {
        self.name
    }

    /// TODO: docs
    #[doc(hidden)]
    pub const fn from_str(name: &'static str) -> Self {
        Self { name }
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn id(&self) -> ModuleId {
        ModuleId::from_module_name(self.name)
    }
}

/// TODO: docs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ModuleId(u64);

impl ModuleId {
    /// TODO: docs
    #[inline]
    pub(crate) fn from_module_name(name: &str) -> Self {
        let mut hasher = std::hash::DefaultHasher::new();
        name.hash(&mut hasher);
        let hash = hasher.finish();
        Self(hash)
    }
}
