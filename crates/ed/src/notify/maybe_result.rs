use core::convert::Infallible;

use crate::notify;

/// TODO: docs
pub trait MaybeResult<T> {
    /// TODO: docs.
    type Error: notify::Error;

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

impl<T, E: notify::Error> MaybeResult<T> for Result<T, E> {
    type Error = E;

    #[inline]
    fn into_result(self) -> Result<T, Self::Error> {
        self
    }
}
