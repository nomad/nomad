use core::any::{self, Any};
use core::cell::Cell;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use std::backtrace::Backtrace;
use std::collections::hash_map::Entry;
use std::panic;

use fxhash::FxHashMap;

use crate::backend::{AgentId, Backend};
use crate::module::{Module, ModuleId};
use crate::notify::{Name, Namespace};
use crate::plugin::{PanicInfo, PanicLocation, Plugin, PluginId};
use crate::{EditorCtx, Shared};

/// TODO: docs.
pub(crate) struct State<B: Backend> {
    backend: B,
    modules: FxHashMap<ModuleId, &'static dyn Any>,
    next_agent_id: AgentId,
    panic_handlers: FxHashMap<PluginId, &'static dyn PanicHandler<B>>,
    panic_hook: PanicHook<B>,
}

/// TODO: docs.
pub(crate) struct StateHandle<B: Backend> {
    inner: Shared<State<B>>,
}

/// TODO: docs.
pub(crate) struct StateMut<'a, B: Backend> {
    state: &'a mut State<B>,
    handle: &'a StateHandle<B>,
}

/// A `PanicHandler` that handles panics by resuming to unwind the stack.
pub(crate) struct ResumeUnwinding;

struct PanicHook<B: Backend> {
    backend: PhantomData<B>,
}

trait PanicHandler<B: Backend> {
    fn handle_panic(&self, info: PanicInfo, ctx: &mut EditorCtx<B>);
}

impl<B: Backend> State<B> {
    #[track_caller]
    #[inline]
    pub(crate) fn add_module<M>(&mut self, module: M) -> &'static M
    where
        M: Module<B>,
    {
        match self.modules.entry(M::id()) {
            Entry::Vacant(entry) => {
                let module = Box::leak(Box::new(module));
                entry.insert(module);
                module
            },
            Entry::Occupied(_) => unreachable!(
                "a module of type {:?} has already been added",
                any::type_name::<M>()
            ),
        }
    }

    #[track_caller]
    #[inline]
    pub(crate) fn add_plugin<P>(&mut self, plugin: P) -> &'static P
    where
        P: Plugin<B>,
    {
        let vacancy = match self.modules.entry(<P as Plugin<_>>::id().into()) {
            Entry::Vacant(entry) => entry,
            Entry::Occupied(_) => unreachable!(
                "a plugin of type {:?} has already been added",
                any::type_name::<P>()
            ),
        };
        let plugin = Box::leak(Box::new(plugin));
        vacancy.insert(plugin);
        self.panic_handlers.insert(<P as Plugin<_>>::id(), plugin);
        plugin
    }

    #[inline]
    pub(crate) fn get_module<M>(&self) -> Option<&'static M>
    where
        M: Module<B>,
    {
        self.modules.get(&M::id()).map(|module| {
            // SAFETY: the ModuleId matched.
            unsafe { downcast_ref_unchecked(*module) }
        })
    }

    #[inline]
    pub(crate) fn new(backend: B) -> Self {
        const RESUME_UNWINDING: &ResumeUnwinding = &ResumeUnwinding;
        Self {
            backend,
            modules: FxHashMap::default(),
            next_agent_id: AgentId::default(),
            panic_handlers: FxHashMap::from_iter(core::iter::once((
                <ResumeUnwinding as Plugin<B>>::id(),
                RESUME_UNWINDING as &'static dyn PanicHandler<B>,
            ))),
            panic_hook: PanicHook::set(),
        }
    }

    #[inline]
    pub(crate) fn next_agent_id(&mut self) -> AgentId {
        self.next_agent_id.post_inc()
    }
}

impl<B: Backend> StateHandle<B> {
    #[inline]
    pub(crate) fn new(backend: B) -> Self {
        Self { inner: Shared::new(State::new(backend)) }
    }

    #[track_caller]
    #[inline]
    pub(crate) fn with_mut<R>(
        &self,
        f: impl FnOnce(StateMut<'_, B>) -> R,
    ) -> R {
        self.inner.with_mut(|state| f(StateMut { state, handle: self }))
    }
}

impl<B: Backend> StateMut<'_, B> {
    #[inline]
    pub(crate) fn as_mut(&mut self) -> StateMut<'_, B> {
        StateMut { state: self.state, handle: self.handle }
    }

    #[inline]
    pub(crate) fn handle(&self) -> StateHandle<B> {
        self.handle.clone()
    }

    #[inline]
    pub(crate) fn handle_panic(
        &mut self,
        namespace: &Namespace,
        plugin_id: PluginId,
        payload: Box<dyn Any + Send>,
    ) {
        let handler = *self
            .panic_handlers
            .get(&plugin_id)
            .expect("no handler matching the given ID");
        let info = self.panic_hook.to_info(payload);
        #[allow(deprecated)]
        let mut ctx = EditorCtx::new(namespace, plugin_id, self.as_mut());
        handler.handle_panic(info, &mut ctx);
    }

    #[track_caller]
    #[inline]
    pub(crate) fn with_ctx<R>(
        &mut self,
        namespace: &Namespace,
        plugin_id: PluginId,
        fun: impl FnOnce(&mut EditorCtx<B>) -> R,
    ) -> Option<R> {
        #[allow(deprecated)]
        let mut ctx = EditorCtx::new(namespace, plugin_id, self.as_mut());
        match panic::catch_unwind(panic::AssertUnwindSafe(|| fun(&mut ctx))) {
            Ok(ret) => Some(ret),
            Err(payload) => {
                self.handle_panic(namespace, plugin_id, payload);
                None
            },
        }
    }
}

impl<B: Backend> PanicHook<B> {
    thread_local! {
        static BACKTRACE: Cell<Option<Backtrace>> = const { Cell::new(None) };
        static LOCATION: Cell<Option<PanicLocation>> = const { Cell::new(None) };
    }

    #[inline]
    fn to_info(&self, payload: Box<dyn Any + Send + 'static>) -> PanicInfo {
        let backtrace = Self::BACKTRACE.with(|b| b.take());
        let location = Self::LOCATION.with(|l| l.take());
        PanicInfo { backtrace, location, payload }
    }

    #[inline]
    fn set() -> Self {
        let prev_hook = B::REINSTATE_PANIC_HOOK.then(panic::take_hook);
        panic::set_hook({
            Box::new(move |info| {
                if let Some(prev) = &prev_hook {
                    prev(info);
                }
                let trace = Backtrace::capture();
                let location = info.location().map(Into::into);
                Self::BACKTRACE.with(move |b| b.set(Some(trace)));
                Self::LOCATION.with(move |l| l.set(location));
            })
        });
        Self { backend: PhantomData }
    }
}

// FIXME: remove once upstream is stabilized.
#[inline]
unsafe fn downcast_ref_unchecked<T: Any>(value: &dyn Any) -> &T {
    debug_assert!(value.is::<T>());
    // SAFETY: caller guarantees that T is the correct type.
    unsafe { &*(value as *const dyn Any as *const T) }
}

impl<B: Backend> Clone for StateHandle<B> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<B: Backend> Deref for State<B> {
    type Target = B;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.backend
    }
}

impl<B: Backend> DerefMut for State<B> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.backend
    }
}

impl<B: Backend> Deref for StateMut<'_, B> {
    type Target = State<B>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.state
    }
}

impl<B: Backend> DerefMut for StateMut<'_, B> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state
    }
}

impl<P, B> PanicHandler<B> for P
where
    P: Plugin<B>,
    B: Backend,
{
    #[inline]
    fn handle_panic(&self, info: PanicInfo, ctx: &mut EditorCtx<B>) {
        Plugin::handle_panic(self, info, ctx);
    }
}

impl<B: Backend> Module<B> for ResumeUnwinding {
    const NAME: Name = "";
    type Config = ();

    fn api(&self, _: &mut crate::module::ApiCtx<B>) {
        unreachable!()
    }
    fn on_new_config(&self, _: Self::Config, _: &mut EditorCtx<B>) {
        unreachable!()
    }
}

impl<B: Backend> Plugin<B> for ResumeUnwinding {
    #[inline]
    fn handle_panic(&self, info: PanicInfo, _: &mut EditorCtx<B>) {
        panic::resume_unwind(info.payload);
    }
}
