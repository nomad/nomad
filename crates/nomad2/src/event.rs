use crate::{Context, Editor};

pub trait Event<E: Editor>: 'static + Ord {
    /// TODO: docs.
    type Args;

    /// TODO: docs.
    type SubscribeRes;

    /// TODO: docs.
    fn subscribe(
        &self,
        emitter: Emitter<Self::Args>,
        ctx: &Context<E>,
    ) -> Self::SubscribeRes;

    /// TODO: docs.
    fn cleanup(&self, sub_res: Self::SubscribeRes, ctx: &Context<E>);
}
