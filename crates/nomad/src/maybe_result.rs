//! TODO: docs

use crate::WarningMsg;

/// TODO: docs
pub trait MaybeResult<T> {
    /// TODO: docs
    type Error: Into<WarningMsg>;

    /// TODO: docs
    fn into_result(self) -> Result<T, Self::Error>;
}

impl<T> MaybeResult<T> for T {
    // TODO: change this to the never type (!) when it becomes stable.
    type Error = core::convert::Infallible;

    #[inline]
    fn into_result(self) -> Result<T, Self::Error> {
        Ok(self)
    }
}

impl<T, E> MaybeResult<T> for Result<T, E>
where
    E: Into<WarningMsg>,
{
    type Error = E;

    #[inline]
    fn into_result(self) -> Result<T, Self::Error> {
        self
    }
}
