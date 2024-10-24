use nvim_oxi::api;

use crate::actor_map::ActorMap;
use crate::autocmd::{AugroupId, AutoCommandMap};
use crate::buf_attach::BufAttachMap;
use crate::buffer_id::BufferId;
use crate::ctx::BufferCtx;
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

#[derive(Default)]
struct CtxInner {
    actor_map: ActorMap,
    augroup_id: NomadAugroupId,
    autocmd_map: AutoCommandMap,
    buf_attach_map: BufAttachMap,
}

#[derive(Copy, Clone)]
struct NomadAugroupId(AugroupId);

impl<'ctx> NeovimCtx<'ctx> {
    /// TODO: docs.
    pub fn into_buffer(self, buffer_id: BufferId) -> Option<BufferCtx<'ctx>> {
        BufferCtx::from_neovim(buffer_id, self)
    }

    /// TODO: docs.
    pub fn reborrow(&self) -> NeovimCtx<'_> {
        NeovimCtx { ctx: self.ctx.as_ref() }
    }

    pub(crate) fn augroup_id(&self) -> AugroupId {
        self.ctx.with_inner(|inner| inner.augroup_id.into())
    }

    pub(crate) fn to_static(&self) -> NeovimCtx<'static> {
        NeovimCtx { ctx: self.ctx.clone().into_owned() }
    }

    pub(crate) fn with_actor_map<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(&mut ActorMap) -> R,
    {
        self.ctx.with_inner(|inner| fun(&mut inner.actor_map))
    }

    pub(crate) fn with_autocmd_map<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(&mut AutoCommandMap) -> R,
    {
        self.ctx.with_inner(|inner| fun(&mut inner.autocmd_map))
    }

    pub(crate) fn with_buf_attach_map<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(&mut BufAttachMap) -> R,
    {
        self.ctx.with_inner(|inner| fun(&mut inner.buf_attach_map))
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

impl Default for NomadAugroupId {
    fn default() -> Self {
        let opts = api::opts::CreateAugroupOpts::builder().clear(true).build();
        let augroup_id =
            api::create_augroup(crate::Nomad::AUGROUP_NAME, &opts)
                .expect("all the arguments are valid")
                .into();
        Self(augroup_id)
    }
}

impl From<NomadAugroupId> for AugroupId {
    fn from(NomadAugroupId(id): NomadAugroupId) -> Self {
        id
    }
}
