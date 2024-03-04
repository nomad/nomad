use core::ops::Deref;

use serde::Deserialize;

use crate::module::{DefaultEnable, Module};

/// TODO: docs
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnableConfig<M: Module> {
    #[serde(default = "default_enable::<M>")]
    enable: bool,

    #[serde(flatten)]
    module_config: M::Config,
}

const fn default_enable<T: DefaultEnable>() -> bool {
    T::ENABLE
}

impl<M: Module> Default for EnableConfig<M> {
    #[inline]
    fn default() -> Self {
        Self {
            enable: default_enable::<M>(),
            module_config: M::Config::default(),
        }
    }
}

impl<M: Module> Deref for EnableConfig<M> {
    type Target = M::Config;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.module_config
    }
}

impl<M: Module> EnableConfig<M> {
    /// TODO: docs
    #[inline]
    pub fn enabled(&self) -> bool {
        self.enable
    }
}
