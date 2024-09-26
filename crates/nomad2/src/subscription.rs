use core::pin::Pin;
use core::task::{Context, Poll};
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use futures_util::Stream;

use crate::{Editor, Event};

/// TODO: docs.
pub struct Subscription<T: Event<E>, E: Editor> {
    rx: Receiver<T::Payload>,
    count: Arc<AtomicU32>,
    ctx: crate::Context<E>,
}

impl<T: Event<E>, E: Editor> Subscription<T, E> {
    pub(crate) fn new(
        rx: Receiver<T::Payload>,
        count: Arc<AtomicU32>,
        ctx: crate::Context<E>,
    ) -> Self {
        Self { rx, count, ctx }
    }
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

pub(crate) fn channel<T>() -> (Emitter<T>, Receiver<T>) {
    todo!();
}

/// TODO: docs.
pub struct Emitter<T> {
    item: T,
}

impl<T> Emitter<T> {
    /// TODO: docs.
    #[inline]
    pub fn send(&self, _: T) {
        todo!();
    }
}

pub(crate) struct Receiver<T> {
    inner: T,
}

impl<T> Receiver<T> {
    pub(crate) fn deactivate(self) -> InactiveReceiver<T> {
        todo!();
    }
}

impl<T> Clone for Receiver<T> {
    #[inline]
    fn clone(&self) -> Self {
        todo!();
    }
}

pub(crate) struct InactiveReceiver<T> {
    inner: T,
}

impl<T> InactiveReceiver<T> {
    pub(crate) fn reactivate(self) -> Receiver<T> {
        todo!();
    }

    pub(crate) fn into_any(self) -> AnyReceiver {
        todo!();
    }
}

impl<T> Clone for InactiveReceiver<T> {
    #[inline]
    fn clone(&self) -> Self {
        todo!();
    }
}

pub(crate) struct AnyReceiver {
    inner: InactiveReceiver<()>,
}

impl AnyReceiver {
    pub(crate) unsafe fn downcast_ref_unchecked<T>(
        &self,
    ) -> &InactiveReceiver<T> {
        todo!();
    }
}
