use core::any::{self, Any, TypeId};
use core::ops::{Deref, DerefMut};
use std::collections::hash_map::Entry;
use std::panic;

use fxhash::FxHashMap;

use crate::backend::Backend;
use crate::module::Module;
use crate::notify::Namespace;
use crate::{NeovimCtx, Shared};

/// TODO: docs.
pub(crate) struct State<B> {
    backend: B,
    modules: FxHashMap<TypeId, &'static dyn Any>,
}

/// TODO: docs.
pub(crate) struct StateHandle<B> {
    inner: Shared<State<B>>,
}

/// TODO: docs.
pub(crate) struct StateMut<'a, B> {
    state: &'a mut State<B>,
    handle: &'a StateHandle<B>,
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
                entry.insert(module);
                module
            },
            Entry::Occupied(_) => unreachable!(
                "a module of type {:?} has already been added",
                any::type_name::<M>()
            ),
        }
    }

    #[inline]
    pub(crate) fn get_module<M>(&self) -> Option<&'static M>
    where
        M: Module<B>,
    {
        self.modules.get(&TypeId::of::<M>()).map(|&module| {
            // SAFETY: the TypeId matched.
            unsafe { downcast_ref_unchecked(module) }
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

    #[inline]
    pub(crate) fn with_ctx<F, R>(
        &mut self,
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
            Err(_payload) => todo!(),
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

impl<B> Clone for StateHandle<B> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<B> Deref for State<B> {
    type Target = B;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.backend
    }
}

impl<B> DerefMut for State<B> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.backend
    }
}

impl<B> Deref for StateMut<'_, B> {
    type Target = State<B>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.state
    }
}

impl<B> DerefMut for StateMut<'_, B> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state
    }
}
