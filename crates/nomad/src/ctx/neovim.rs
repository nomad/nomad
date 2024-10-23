use nvim_oxi::api;

use crate::actor_map::ActorMap;
use crate::autocmd::AugroupId;
use crate::{Boo, Shared};

/// TODO: docs.
#[derive(Default, Clone)]
pub struct NeovimCtx<'ctx> {
    ctx: Boo<'ctx, Ctx>,
}

#[derive(Default, Clone)]
struct Ctx {
    inner: Shared<CtxInner>,
}

struct CtxInner {
    actor_map: ActorMap,
    augroup_id: AugroupId,
}

impl NeovimCtx<'_> {
    pub(crate) fn augroup_id(&self) -> AugroupId {
        self.ctx.with_inner(|inner| inner.augroup_id)
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

impl Default for CtxInner {
    fn default() -> Self {
        let opts = api::opts::CreateAugroupOpts::builder().clear(true).build();
        let augroup_id = api::create_augroup(crate::Nomad::NAME, &opts)
            .expect("all the arguments are valid")
            .into();
        Self { actor_map: ActorMap::default(), augroup_id }
    }
}
