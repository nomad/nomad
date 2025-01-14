use core::convert::Infallible;

use crate::notify::{Level, Message, Source};

/// TODO: docs.
pub trait Error {
    /// TODO: docs.
    fn to_message(&self, source: Source) -> Option<(Level, Message)>;
}

impl Error for Infallible {
    fn to_message(&self, _: Source) -> Option<(Level, Message)> {
        unreachable!()
    }
}

impl<T: Error> Error for &T {
    #[inline]
    fn to_message(&self, source: Source) -> Option<(Level, Message)> {
        (**self).to_message(source)
    }
}
