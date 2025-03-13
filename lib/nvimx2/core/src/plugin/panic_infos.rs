use core::any::Any;
use core::fmt;
use core::panic::Location;
use std::backtrace::Backtrace;

use smol_str::SmolStr;

/// TODO: docs.
#[non_exhaustive]
pub struct PanicInfo {
    /// TODO: docs.
    pub backtrace: Option<Backtrace>,

    /// TODO: docs.
    pub location: Option<PanicLocation>,

    /// TODO: docs.
    pub payload: Box<dyn Any + Send + 'static>,
}

/// TODO: docs.
pub struct PanicLocation {
    column: u32,
    file: SmolStr,
    line: u32,
}

impl PanicInfo {
    /// TODO: docs.
    #[inline]
    pub fn payload_as_str(&self) -> Option<&str> {
        self.payload
            .downcast_ref::<String>()
            .map(|s| &**s)
            .or_else(|| self.payload.downcast_ref::<&str>().copied())
    }
}

impl PanicLocation {
    /// TODO: docs.
    #[inline]
    pub fn column(&self) -> u32 {
        self.column
    }

    /// TODO: docs.
    #[inline]
    pub fn file(&self) -> &str {
        &self.file
    }

    /// TODO: docs.
    #[inline]
    pub fn line(&self) -> u32 {
        self.line
    }
}

impl fmt::Display for PanicLocation {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

impl From<&Location<'_>> for PanicLocation {
    #[inline]
    fn from(location: &Location<'_>) -> Self {
        Self {
            column: location.column(),
            file: location.file().into(),
            line: location.line(),
        }
    }
}
