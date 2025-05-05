use core::convert::Infallible;
use core::fmt;

use crate::notify;

/// TODO: docs.
pub trait Value: 'static {
    /// TODO: docs.
    type MapAccess<'a>: MapAccess<Value = Self>;

    /// TODO: docs.
    type MapAccessError<'a>: notify::Error
    where
        Self: 'a;

    /// TODO: docs.
    fn map_access(
        &mut self,
    ) -> Result<Self::MapAccess<'_>, Self::MapAccessError<'_>>;
}

/// TODO: docs.
pub trait MapAccess {
    /// TODO: docs.
    type Key<'a>: Key
    where
        Self: 'a;

    /// TODO: docs.
    type Value;

    /// TODO: docs.
    fn next_key(&mut self) -> Option<Self::Key<'_>>;

    /// TODO: docs.
    fn take_next_value(&mut self) -> Self::Value;
}

/// TODO: docs.
pub trait Key: fmt::Debug {
    /// TODO: docs.
    type AsStrError<'a>: notify::Error
    where
        Self: 'a;

    /// TODO: docs.
    fn as_str(&self) -> Result<&str, Self::AsStrError<'_>>;
}

impl<MA: MapAccess> MapAccess for &mut MA {
    type Key<'a>
        = MA::Key<'a>
    where
        Self: 'a;

    type Value = MA::Value;

    #[inline]
    fn next_key(&mut self) -> Option<Self::Key<'_>> {
        MA::next_key(self)
    }

    #[inline]
    fn take_next_value(&mut self) -> Self::Value {
        MA::take_next_value(self)
    }
}

impl Key for &str {
    type AsStrError<'a>
        = Infallible
    where
        Self: 'a;

    #[inline]
    fn as_str(&self) -> Result<&str, Self::AsStrError<'_>> {
        Ok(self)
    }
}
