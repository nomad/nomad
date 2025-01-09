use core::convert::Infallible;

use crate::backend::Backend;
use crate::notify::{Level, Message, Source};
use crate::plugin::Plugin;

/// TODO: docs.
pub trait Error<B: Backend> {
    /// TODO: docs.
    fn to_message<P>(&self, source: Source) -> Option<(Level, Message)>
    where
        P: Plugin<B>;
}

impl<B: Backend> Error<B> for Infallible {
    fn to_message<P>(&self, _: Source) -> Option<(Level, Message)>
    where
        P: Plugin<B>,
    {
        unreachable!()
    }
}

impl<T: Error<B>, B: Backend> Error<B> for &T {
    #[inline]
    fn to_message<P>(&self, source: Source) -> Option<(Level, Message)>
    where
        P: Plugin<B>,
    {
        (&**self).to_message::<P>(source)
    }
}
