use alloc::rc::Rc;
use core::any::Any;

use crate::{Context, Editor, Emitter};

/// TODO: docs.
pub trait Event<E: Editor>: 'static + Ord {
    /// TODO: docs.
    type Payload;

    /// The result of subscribing to this event. This can be used to pass state
    /// from `subscribe` to `unsubscribe`.
    type SubscribeCtx;

    /// TODO: docs.
    fn subscribe(
        &self,
        emitter: Emitter<Self::Payload>,
        ctx: &Context<E>,
    ) -> Self::SubscribeCtx;

    /// TODO: docs.
    fn unsubscribe(&self, subscribe_ctx: Self::SubscribeCtx, ctx: &Context<E>);
}

#[derive(Clone)]
pub(crate) struct AnyEvent {
    inner: Rc<dyn Any>,
}

impl AnyEvent {
    #[inline]
    pub(crate) fn downcast_ref<T: Event<E>, E: Editor>(&self) -> &T {
        match self.inner.downcast_ref() {
            Some(event) => event,
            None => panic!("downcasting AnyEvent to the wrong event type"),
        }
    }

    #[inline]
    pub(crate) fn new<T: Event<E>, E: Editor>(event: T) -> Self {
        Self { inner: Rc::new(event) }
    }

    #[inline]
    pub(crate) fn ref_count(&self) -> usize {
        Rc::strong_count(&self.inner)
    }
}
