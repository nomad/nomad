//! TODO: docs

use core::convert::Infallible;

use crate::diagnostics::DiagnosticMessage;

/// TODO: docs
pub trait MaybeResult<T> {
    /// TODO: docs
    type Error: Into<DiagnosticMessage>;

    /// TODO: docs
    fn into_result(self) -> Result<T, Self::Error>;
}

impl<T> MaybeResult<T> for T {
    // FIXME: change this to the never type (!) when it becomes stable.
    type Error = Infallible;

    #[inline]
    fn into_result(self) -> Result<T, Self::Error> {
        Ok(self)
    }
}

impl<T, E> MaybeResult<T> for Result<T, E>
where
    E: Into<DiagnosticMessage>,
{
    type Error = E;

    #[inline]
    fn into_result(self) -> Result<T, Self::Error> {
        self
    }
}
