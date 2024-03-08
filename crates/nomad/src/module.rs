//! TODO: docs

use core::future::Future;
use core::hash::{Hash, Hasher};

pub use macros::module_name;
use serde::de::DeserializeOwned;

use crate::prelude::*;

/// TODO: docs
pub trait Module: 'static + Sized {
    /// TODO: docs
    const NAME: ModuleName;

    /// TODO: docs
    type Config: Default + DeserializeOwned;

    /// TODO: docs
    fn init(config: Get<Self::Config>, ctx: &InitCtx) -> Api<Self>;

    /// TODO: docs
    fn run(
        &self,
        // ctx: &mut SetCtx,
    ) -> impl Future<Output = impl MaybeResult<()>>;
}

/// TODO: docs
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleName {
    name: &'static str,
}

impl core::fmt::Debug for ModuleName {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ModuleName").field(&self.name).finish()
    }
}

impl core::fmt::Display for ModuleName {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
