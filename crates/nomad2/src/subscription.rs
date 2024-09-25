use core::pin::Pin;
use core::task::{Context, Poll};

use futures_util::Stream;

use crate::{Editor, Event};

/// TODO: docs.
pub struct Subscription<T: Event<E>, E: Editor> {
    event: T,
    editor: E,
}

impl<T: Event<E>, E: Editor> Stream for Subscription<T, E> {
    type Item = T::Payload;

    #[inline]
    fn poll_next(
        self: Pin<&mut Self>,
        _ctx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        todo!();
    }
}
