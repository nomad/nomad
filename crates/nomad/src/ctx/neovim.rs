use crate::actor_map::ActorMap;
use crate::autocmd::AugroupId;
use crate::{Boo, Shared};

/// TODO: docs.
#[derive(Clone)]
pub struct NeovimCtx<'ctx> {
    ctx: Boo<'ctx, Ctx>,
}

#[derive(Default, Clone)]
struct Ctx {
    inner: Shared<CtxInner>,
}

#[derive(Default)]
struct CtxInner {
    actor_map: ActorMap,
}

impl NeovimCtx<'_> {
    pub(crate) fn augroup_id(&self) -> AugroupId {
        todo!();
    }

    pub(crate) fn as_ref(&self) -> NeovimCtx<'_> {
        NeovimCtx { ctx: self.ctx.as_ref() }
    }

    pub(crate) fn to_static(&self) -> NeovimCtx<'static> {
        NeovimCtx { ctx: self.ctx.clone().into_owned() }
    }

    pub(in crate::ctx) fn with_actor_map<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(&mut ActorMap) -> R,
    {
        self.ctx.with_inner(|inner| fun(&mut inner.actor_map))
    }
}

impl Ctx {
    fn with_inner<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(&mut CtxInner) -> R,
    {
        self.inner.with_mut(|inner| fun(inner))
    }
}
