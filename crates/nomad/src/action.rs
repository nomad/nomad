//! TODO: docs

use std::hash::{Hash, Hasher};

use crate::{MaybeFuture, MaybeResult, Module};

/// TODO: docs
pub trait Action<M: Module>: 'static {
    /// TODO: docs
    const NAME: ActionName;

    /// TODO: docs
    type Args;

    /// TODO: docs
    //
    // NOTE: this can be removed entirely once we have RTN
    // (https://github.com/rust-lang/rust/issues/109417).
    type Return;

    /// TODO: docs
    fn execute(
        &mut self,
        args: Self::Args,
    ) -> impl MaybeFuture<Output = impl MaybeResult<Self::Return>>;
}

/// TODO: docs
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionName {
    name: &'static str,
}

impl core::fmt::Debug for ActionName {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ActionName").field(&self.name).finish()
    }
}

impl core::fmt::Display for ActionName {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.name)
    }
}

impl AsRef<str> for ActionName {
    #[inline]
    fn as_ref(&self) -> &str {
        self.name
    }
}

impl ActionName {
    /// TODO: docs
    #[inline]
    pub(crate) fn as_str(&self) -> &'static str {
        self.name
    }

    #[doc(hidden)]
    pub const fn from_str(name: &'static str) -> Self {
        Self { name }
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn id(&self) -> ActionId {
        ActionId::from_action_name(self.name)
    }
}

/// TODO: docs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ActionId(u64);

impl ActionId {
    /// TODO: docs
    #[inline]
    pub(crate) fn from_action_name(name: &str) -> Self {
        let mut hasher = std::hash::DefaultHasher::new();
        name.hash(&mut hasher);
        let hash = hasher.finish();
        Self(hash)
    }
}
