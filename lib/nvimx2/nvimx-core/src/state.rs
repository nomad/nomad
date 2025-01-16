use core::any::{self, Any, TypeId};
use core::ops::{Deref, DerefMut};
use std::collections::hash_map::Entry;
use std::panic;

use fxhash::FxHashMap;

use crate::backend::Backend;
use crate::module::Module;
use crate::notify::Namespace;
use crate::plugin::Plugin;
use crate::{NeovimCtx, Shared};

/// TODO: docs.
pub(crate) struct State<B: Backend> {
    backend: B,
    modules: FxHashMap<TypeId, ModuleState<B>>,
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

struct ModuleState<B: Backend> {
    module: &'static dyn Any,
    panic_handler: Option<&'static dyn PanicHandler<B>>,
}

trait PanicHandler<B: Backend> {
    fn handle_panic(
        &self,
        payload: Box<dyn Any + Send + 'static>,
        ctx: &mut NeovimCtx<B>,
    );
}

impl<B: Backend> State<B> {
    #[track_caller]
    #[inline]
    pub(crate) fn add_module<M>(&mut self, module: M) -> &'static M
    where
        M: Module<B>,
    {
        match self.modules.entry(TypeId::of::<M>()) {
            Entry::Vacant(entry) => {
                let module = Box::leak(Box::new(module));
                entry.insert(ModuleState { module, panic_handler: None });
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
        match self.modules.entry(TypeId::of::<P>()) {
            Entry::Vacant(entry) => {
                let plugin = Box::leak(Box::new(plugin));
                entry.insert(ModuleState {
                    module: plugin,
                    panic_handler: Some(plugin),
                });
                plugin
            },
            Entry::Occupied(_) => unreachable!(
                "a plugin of type {:?} has already been added",
                any::type_name::<P>()
            ),
        }
    }
    #[inline]
    pub(crate) fn get_module<M>(&self) -> Option<&'static M>
    where
        M: Module<B>,
    {
        self.modules.get(&TypeId::of::<M>()).map(|module_state| {
            // SAFETY: the TypeId matched.
            unsafe { downcast_ref_unchecked(module_state.module) }
        })
    }

    #[inline]
    pub(crate) fn new(backend: B) -> Self {
        Self { backend, modules: FxHashMap::default() }
    }
}

impl<B: Backend> StateHandle<B> {
    #[inline]
    pub(crate) fn new(backend: B) -> Self {
        Self { inner: Shared::new(State::new(backend)) }
    }

    #[track_caller]
    #[inline]
    pub(crate) fn with_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(StateMut<'_, B>) -> R,
    {
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

    #[track_caller]
    #[inline]
    pub(crate) fn with_ctx<F, R>(
        &mut self,
        plugin_id: TypeId,
        namespace: &Namespace,
        fun: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut NeovimCtx<B>) -> R,
    {
        #[allow(deprecated)]
        let mut ctx = NeovimCtx::new(namespace, self.as_mut());
        match panic::catch_unwind(panic::AssertUnwindSafe(|| fun(&mut ctx))) {
            Ok(ret) => Some(ret),
            Err(payload) => {
                let handler = self
                    .modules
                    .get(&plugin_id)
                    .expect("no plugin matching the given TypeId")
                    .panic_handler
                    .expect("TypeId is of a Module, not a Plugin");
                #[allow(deprecated)]
                let mut ctx = NeovimCtx::new(namespace, self.as_mut());
                handler.handle_panic(payload, &mut ctx);
                None
            },
        }
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
    fn handle_panic(
        &self,
        payload: Box<dyn Any + Send + 'static>,
        ctx: &mut NeovimCtx<B>,
    ) {
        Plugin::handle_panic(self, payload, ctx);
    }
}
