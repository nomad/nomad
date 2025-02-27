use core::convert::Infallible;

use crate::notify::{Level, Message};

/// TODO: docs.
pub trait Error {
    /// TODO: docs.
    fn to_message(&self) -> (Level, Message);
}

impl Error for Infallible {
    fn to_message(&self) -> (Level, Message) {
        unreachable!()
    }
}

impl<T: Error> Error for &T {
    #[inline]
    fn to_message(&self) -> (Level, Message) {
        (**self).to_message()
    }
}

impl Error for Box<dyn core::error::Error> {
    #[inline]
    fn to_message(&self) -> (Level, Message) {
        (Level::Error, Message::from_display(self))
    }
}
