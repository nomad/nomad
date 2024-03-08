//! TODO: docs

pub use macros::action_name;
use serde::de::DeserializeOwned;

use crate::prelude::{Module, SetCtx};

/// TODO: docs
pub trait Action<M: Module>: 'static {
    /// TODO: docs
    const NAME: ActionName;

    /// TODO: docs
    type Args: DeserializeOwned;

    /// TODO: docs
    fn execute(&self, args: Self::Args, ctx: &mut SetCtx);
}

/// TODO: docs
#[derive(PartialEq, Eq, Hash)]
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
}
