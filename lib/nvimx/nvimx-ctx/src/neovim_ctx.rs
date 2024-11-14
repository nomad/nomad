use core::future::Future;

use nvim_oxi::api::{self, opts};
use nvimx_common::Shared;
use nvimx_executor::{Executor as LocalExecutor, JoinHandle};

use crate::actor_id::ActorId;
use crate::actor_map::ActorMap;
use crate::autocmd::{AugroupId, AutoCommandMap};
use crate::boo::Boo;
use crate::buf_attach::BufAttachMap;
use crate::buffer_ctx::BufferCtx;
use crate::buffer_id::BufferId;
use crate::decoration_provider::{DecorationProvider, NamespaceId};

/// TODO: docs.
#[derive(Clone)]
pub struct NeovimCtx<'ctx> {
    ctx: Boo<'ctx, Shared<Ctx>>,
}

struct Ctx {
    actor_map: ActorMap,
    augroup_id: AugroupId,
    autocmd_map: AutoCommandMap,
    buf_attach_map: BufAttachMap,
    decoration_provider: Option<DecorationProvider>,
    local_executor: LocalExecutor,
    namespace_id: NamespaceId,
    next_actor_id: ActorId,
}

impl<'ctx> NeovimCtx<'ctx> {
    /// TODO: docs.
    pub fn into_buffer(self, buffer_id: BufferId) -> Option<BufferCtx<'ctx>> {
        BufferCtx::from_neovim(buffer_id, self)
    }

    /// TODO: docs.
    pub fn next_actor_id(&self) -> ActorId {
        self.ctx.with_mut(|ctx| ctx.next_actor_id.post_inc())
    }

    /// TODO: docs.
    pub fn reborrow(&self) -> NeovimCtx<'_> {
        NeovimCtx { ctx: self.ctx.as_ref() }
    }

    /// TODO: docs.
    pub fn spawn<F, Fut>(&self, callback: F) -> JoinHandle<Fut::Output>
    where
        F: FnOnce(NeovimCtx<'static>) -> Fut,
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        let future = callback(self.to_static());
        self.ctx.with_mut(move |ctx| ctx.local_executor.spawn(future))
    }

    /// TODO: docs.
    pub fn to_static(&self) -> NeovimCtx<'static> {
        NeovimCtx { ctx: self.ctx.clone().into_owned() }
    }

    pub(crate) fn augroup_id(&self) -> AugroupId {
        self.ctx.with_mut(|ctx| ctx.augroup_id)
    }

    pub(crate) fn namespace_id(&self) -> NamespaceId {
        self.ctx.with_mut(|ctx| ctx.namespace_id)
    }

    pub(crate) fn with_actor_map<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(&mut ActorMap) -> R,
    {
        self.ctx.with_mut(|ctx| fun(&mut ctx.actor_map))
    }

    pub(crate) fn with_autocmd_map<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(&mut AutoCommandMap) -> R,
    {
        self.ctx.with_mut(|ctx| fun(&mut ctx.autocmd_map))
    }

    pub(crate) fn with_buf_attach_map<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(&mut BufAttachMap) -> R,
    {
        self.ctx.with_mut(|ctx| fun(&mut ctx.buf_attach_map))
    }

    pub(crate) fn with_decoration_provider<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(&mut DecorationProvider) -> R,
    {
        self.ctx.with_mut(|ctx| {
            let provider = ctx.decoration_provider.get_or_insert_with(|| {
                DecorationProvider::new(self.to_static())
            });
            fun(provider)
        })
    }
}

impl Ctx {
    fn new(augroup_name: &str, namespace_name: &str) -> Self {
        let augroup_id = {
            let opts = opts::CreateAugroupOpts::builder().clear(true).build();
            api::create_augroup(augroup_name, &opts)
                .expect("all the arguments are valid")
                .into()
        };

        Self {
            actor_map: ActorMap::default(),
            augroup_id,
            autocmd_map: AutoCommandMap::default(),
            buf_attach_map: BufAttachMap::default(),
            decoration_provider: None,
            local_executor: LocalExecutor::register(),
            namespace_id: NamespaceId::new(namespace_name),
            next_actor_id: ActorId::new(1),
        }
    }
}
