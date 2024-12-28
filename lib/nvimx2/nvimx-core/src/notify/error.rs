use core::convert::Infallible;

use super::{Level, Message};

/// TODO: docs.
pub trait Error {
    /// TODO: docs.
    fn to_level(&self) -> Option<Level>;

    /// TODO: docs.
    fn to_message(&self) -> Message;
}

impl Error for Infallible {
    fn to_level(&self) -> Option<Level> {
        unreachable!()
    }

    fn to_message(&self) -> Message {
        unreachable!()
    }
}

impl<T: Error> Error for &T {
    fn to_level(&self) -> Option<Level> {
        (**self).to_level()
    }

    fn to_message(&self) -> Message {
        (**self).to_message()
    }
}

impl Error for Box<dyn Error> {
    fn to_level(&self) -> Option<Level> {
        (**self).to_level()
    }

    fn to_message(&self) -> Message {
        (**self).to_message()
    }
}
