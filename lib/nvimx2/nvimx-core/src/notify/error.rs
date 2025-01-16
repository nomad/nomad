use core::convert::Infallible;

use crate::notify::{Level, Message, Namespace};

/// TODO: docs.
pub trait Error {
    /// TODO: docs.
    fn to_message(&self, namespace: &Namespace) -> (Level, Message);
}

impl Error for Infallible {
    fn to_message(&self, _: &Namespace) -> (Level, Message) {
        unreachable!()
    }
}

impl<T: Error> Error for &T {
    #[inline]
    fn to_message(&self, namespace: &Namespace) -> (Level, Message) {
        (**self).to_message(namespace)
    }
}
