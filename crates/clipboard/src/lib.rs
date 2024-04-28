//! TODO: docs

use core::fmt::{self, Display};
use std::error::Error as StdError;

use copypasta::{ClipboardContext, ClipboardProvider};

/// TODO: docs
#[inline]
pub fn get() -> Result<String, ClipboardError> {
    ClipboardContext::new()
        .and_then(|mut ctx| ctx.get_contents())
        .map_err(ClipboardError::new_get)
}

/// TODO: docs
#[inline]
pub fn set<T: Display>(value: T) -> Result<(), ClipboardError> {
    ClipboardContext::new()
        .and_then(|mut ctx| ctx.set_contents(value.to_string()))
        .map_err(ClipboardError::new_set)
}

/// TODO: docs
#[derive(Debug)]
pub struct ClipboardError {
    inner: Box<dyn StdError + Send + Sync + 'static>,
    kind: ClipboardErrorKind,
}

impl ClipboardError {
    #[inline]
    fn new_get(inner: Box<dyn StdError + Send + Sync + 'static>) -> Self {
        Self { kind: ClipboardErrorKind::Get, inner }
    }

    #[inline]
    fn new_set(inner: Box<dyn StdError + Send + Sync + 'static>) -> Self {
        Self { kind: ClipboardErrorKind::Set, inner }
    }
}

impl Display for ClipboardError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "couldn't {} clipboard: {}", self.kind, self.inner)
    }
}

impl StdError for ClipboardError {}

impl PartialEq for ClipboardError {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.inner.to_string() == other.inner.to_string()
    }
}

impl Eq for ClipboardError {}

#[derive(Debug, Eq, PartialEq)]
enum ClipboardErrorKind {
    Get,
    Set,
}

impl Display for ClipboardErrorKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Get => write!(f, "get"),
            Self::Set => write!(f, "set"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(
        all(target_os = "linux", feature = "__ci"),
        ignore = "fails on headless X11"
    )]
    fn clipboard_set_get_cycle() {
        for idx in 0..10 {
            set(idx).unwrap();
            assert_eq!(get().unwrap(), idx.to_string());
        }
    }
}
