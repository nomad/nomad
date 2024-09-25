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
