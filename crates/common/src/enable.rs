use core::ops::Deref;

/// TODO: docs
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Enable<T> {
    #[serde(default = "yes")]
    enable: bool,

    #[serde(flatten)]
    config: T,
}

fn yes() -> bool {
    true
}

impl<T: Default> Default for Enable<T> {
    fn default() -> Self {
        Self { enable: true, config: T::default() }
    }
}

impl<T> Deref for Enable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl<T> Enable<T> {
    pub fn enable(&self) -> bool {
        self.enable
    }

    pub fn enable_mut(&mut self) -> &mut bool {
        &mut self.enable
    }

    pub fn inner(&self) -> &T {
        self.deref()
    }

    pub fn into_inner(self) -> T {
        self.config
    }
}
