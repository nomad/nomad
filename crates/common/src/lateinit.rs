use std::ops::{Deref, DerefMut};

/// TODO: docs
#[derive(Clone)]
pub struct LateInit<T> {
    inner: Option<T>,
}

impl<T> Default for LateInit<T> {
    fn default() -> Self {
        Self { inner: None }
    }
}

impl<T> LateInit<T> {
    /// TODO: docs
    pub fn init(&mut self, value: T) {
        self.inner = Some(value);
    }

    pub fn into_inner(mut self) -> T {
        self.inner.take().unwrap()
    }

    pub fn new() -> Self {
        Self { inner: None }
    }
}

impl<T> Deref for LateInit<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl<T> DerefMut for LateInit<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().unwrap()
    }
}
