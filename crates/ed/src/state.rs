use core::any::{self, Any};
use core::cell::Cell;
use core::marker::PhantomData;
use core::num::NonZeroU64;
use core::ops::{Deref, DerefMut};
use std::backtrace::Backtrace;
use std::collections::hash_map::Entry;
use std::panic;

use fxhash::FxHashMap;

use crate::context::BorrowedInner;
use crate::module::{Module, ModuleId};
use crate::notify::{Name, Namespace};
use crate::plugin::{PanicInfo, PanicLocation, Plugin, PluginId};
use crate::{AgentId, Borrowed, Context, Editor, Shared};

/// TODO: docs.
#[doc(hidden)]
pub struct State<Ed: Editor> {
    editor: Ed,
    modules: FxHashMap<ModuleId, &'static dyn Any>,
    next_agent_id: AgentId,
    panic_handlers: FxHashMap<PluginId, &'static dyn PanicHandler<Ed>>,
    panic_hook: PanicHook<Ed>,
}

/// TODO: docs.
pub(crate) struct StateHandle<Ed: Editor> {
    inner: Shared<State<Ed>>,
}

/// TODO: docs.
pub(crate) struct StateMut<'a, Ed: Editor> {
    state: &'a mut State<Ed>,
    handle: &'a StateHandle<Ed>,
}

/// A `PanicHandler` that handles panics by resuming to unwind the stack.
pub(crate) struct ResumeUnwinding;

struct PanicHook<Ed: Editor> {
    editor: PhantomData<Ed>,
}

trait PanicHandler<Ed: Editor> {
    fn handle_panic(
        &self,
        info: PanicInfo,
        ctx: &mut Context<Ed, Borrowed<'_>>,
    );
}

impl<Ed: Editor> State<Ed> {
    #[track_caller]
    #[inline]
    pub(crate) fn add_module<M>(&mut self, module: M) -> &'static M
    where
        M: Module<Ed>,
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
        P: Plugin<Ed>,
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
        M: Module<Ed>,
    {
        self.modules.get(&M::id()).map(|module| {
            // SAFETY: the ModuleId matched.
            unsafe { downcast_ref_unchecked(*module) }
        })
    }

    #[inline]
    pub(crate) fn handle_panic(
        payload: Box<dyn Any + Send>,
        ctx: &mut Context<Ed, Borrowed<'_>>,
    ) {
        let plugin_id = ctx.plugin_id();
        let this = ctx.state_mut();
        let handler = *this
            .panic_handlers
            .get(&plugin_id)
            .expect("no handler matching the given ID");
        let info = this.panic_hook.to_info(payload);
        handler.handle_panic(info, ctx);
    }

    #[inline]
    pub(crate) fn new(editor: Ed) -> Self {
        const RESUME_UNWINDING: &ResumeUnwinding = &ResumeUnwinding;
        Self {
            panic_hook: PanicHook::set(&editor),
            editor,
            modules: FxHashMap::default(),
            next_agent_id: AgentId::new(NonZeroU64::new(1).expect("not zero")),
            panic_handlers: FxHashMap::from_iter(core::iter::once((
                <ResumeUnwinding as Plugin<Ed>>::id(),
                RESUME_UNWINDING as &'static dyn PanicHandler<Ed>,
            ))),
        }
    }

    #[inline]
    pub(crate) fn next_agent_id(&mut self) -> AgentId {
        self.next_agent_id.post_inc()
    }
}

impl<Ed: Editor> StateHandle<Ed> {
    #[inline]
    pub(crate) fn new(editor: Ed) -> Self {
        Self { inner: Shared::new(State::new(editor)) }
    }

    #[track_caller]
    #[inline]
    pub(crate) fn with_mut<R>(
        &self,
        f: impl FnOnce(StateMut<'_, Ed>) -> R,
    ) -> R {
        self.inner.with_mut(|state| f(StateMut { state, handle: self }))
    }
}

impl<Ed: Editor> StateMut<'_, Ed> {
    #[inline]
    pub(crate) fn as_mut(&mut self) -> StateMut<'_, Ed> {
        StateMut { state: self.state, handle: self.handle }
    }

    #[inline]
    pub(crate) fn handle(&self) -> StateHandle<Ed> {
        self.handle.clone()
    }

    #[track_caller]
    #[inline]
    pub(crate) fn with_ctx<R>(
        &mut self,
        namespace: &Namespace,
        plugin_id: PluginId,
        fun: impl FnOnce(&mut Context<Ed, Borrowed<'_>>) -> R,
    ) -> Option<R> {
        let mut ctx = Context::new(BorrowedInner {
            namespace,
            plugin_id,
            state_handle: &self.handle.inner,
            state: self.state,
        });
        match panic::catch_unwind(panic::AssertUnwindSafe(|| fun(&mut ctx))) {
            Ok(ret) => Some(ret),
            Err(payload) => {
                State::handle_panic(payload, &mut ctx);
                None
            },
        }
    }
}

impl<Ed: Editor> PanicHook<Ed> {
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
    fn set(ed: &Ed) -> Self {
        let prev_hook = ed.reinstate_panic_hook().then(panic::take_hook);
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
        Self { editor: PhantomData }
    }
}

// FIXME: remove once upstream is stabilized.
#[inline]
unsafe fn downcast_ref_unchecked<T: Any>(value: &dyn Any) -> &T {
    debug_assert!(value.is::<T>());
    // SAFETY: caller guarantees that T is the correct type.
    unsafe { &*(value as *const dyn Any as *const T) }
}

impl<Ed: Editor> Clone for StateHandle<Ed> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<Ed: Editor> Deref for State<Ed> {
    type Target = Ed;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.editor
    }
}

impl<Ed: Editor> DerefMut for State<Ed> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.editor
    }
}

impl<Ed: Editor> Deref for StateMut<'_, Ed> {
    type Target = State<Ed>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.state
    }
}

impl<Ed: Editor> DerefMut for StateMut<'_, Ed> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state
    }
}

impl<P, Ed> PanicHandler<Ed> for P
where
    P: Plugin<Ed>,
    Ed: Editor,
{
    #[inline]
    fn handle_panic(
        &self,
        info: PanicInfo,
        _: &mut Context<Ed, Borrowed<'_>>,
    ) {
        panic::resume_unwind(info.payload);
    }
}

impl<Ed: Editor> Module<Ed> for ResumeUnwinding {
    const NAME: Name = "";
    type Config = ();

    fn api(&self, _: &mut crate::module::ApiCtx<Ed>) {
        unreachable!()
    }
    fn on_new_config(
        &self,
        _: Self::Config,
        _: &mut Context<Ed, Borrowed<'_>>,
    ) {
        unreachable!()
    }
}

impl<Ed: Editor> Plugin<Ed> for ResumeUnwinding {
    #[inline]
    fn handle_panic(
        &self,
        info: PanicInfo,
        ctx: &mut Context<Ed, Borrowed<'_>>,
    ) {
        PanicHandler::handle_panic(self, info, ctx);
    }
}
