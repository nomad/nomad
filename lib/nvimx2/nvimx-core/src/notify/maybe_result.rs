use core::convert::Infallible;

use crate::backend::Backend;
use crate::notify;

/// TODO: docs
pub trait MaybeResult<T, B: Backend> {
    /// TODO: docs.
    type Error: notify::Error<B> + 'static;

    /// TODO: docs
    fn into_result(self) -> Result<T, Self::Error>;
}

impl<T, B: Backend> MaybeResult<T, B> for T {
    // FIXME: change this to the never type (!) when it becomes stable.
    type Error = Infallible;

    #[inline]
    fn into_result(self) -> Result<T, Self::Error> {
        Ok(self)
    }
}

impl<T, E: notify::Error<B> + 'static, B: Backend> MaybeResult<T, B>
    for Result<T, E>
{
    type Error = E;

    #[inline]
    fn into_result(self) -> Result<T, Self::Error> {
        self
    }
}
