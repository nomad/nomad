use core::pin::Pin;
use core::task::Poll;

use futures_util::Stream;

use crate::context::SubscriptionState;
use crate::event::AnyEvent;
use crate::{Context, Editor, Event};

/// TODO: docs.
pub struct Subscription<T: Event<E>, E: Editor> {
    /// Used to remove the state from the context when the last subscription is
    /// dropped.
    event: AnyEvent,

    /// TODO: docs.
    rx: Receiver<T::Payload>,

    /// TODO: docs.
    ctx: Context<E>,
}

impl<T: Event<E>, E: Editor> Subscription<T, E> {
    /// TODO: docs.
    pub async fn recv(&mut self) -> &T::Payload {
        self.rx.recv().await
    }

    pub(crate) fn new(
        event: AnyEvent,
        rx: Receiver<T::Payload>,
        ctx: Context<E>,
    ) -> Self {
        Self { event, rx, ctx }
    }
}

impl<T: Event<E>, E: Editor> Stream for Subscription<T, E>
where
    T::Payload: Clone,
{
    type Item = T::Payload;

    fn poll_next(
        self: Pin<&mut Self>,
        _ctx: &mut core::task::Context,
    ) -> Poll<Option<Self::Item>> {
        todo!();
    }
}

impl<T: Event<E>, E: Editor> Drop for Subscription<T, E> {
    fn drop(&mut self) {
        // The `Context` owns another instance of the event, so if the ref
        // count reaches 2, it means this is the last subscription.
        if self.event.ref_count() == 2 {
            let SubscriptionState { sub_ctx, .. } = self
                .ctx
                .remove_subscription(self.event.downcast_ref::<T, E>())
                .expect("ref count is 2");

            let sub_ctx = sub_ctx
                .downcast::<T::SubscribeCtx>()
                .expect("sub_ctx contains the correct type");

            self.event.downcast_mut::<T, E>().unsubscribe(*sub_ctx, &self.ctx);
        }
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
    pub fn send(&self, _: T) {
        todo!();
    }
}

/// TODO: docs.
pub(crate) struct Receiver<T> {
    inner: T,
}

impl<T> Receiver<T> {
    pub(crate) fn deactivate(self) -> InactiveReceiver<T> {
        todo!();
    }

    async fn recv(&mut self) -> &T {
        todo!();
    }
}

impl<T> Clone for Receiver<T> {
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
