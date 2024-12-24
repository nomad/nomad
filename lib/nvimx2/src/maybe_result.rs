use core::convert::Infallible;
use core::error::Error;

/// TODO: docs
pub trait MaybeResult<T> {
    /// TODO: docs
    type Error: Error;

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
    E: Error,
{
    type Error = E;

    #[inline]
    fn into_result(self) -> Result<T, Self::Error> {
        self
    }
}
