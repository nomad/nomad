use core::any::{self, Any, TypeId};
use std::collections::hash_map::Entry;

use fxhash::FxHashMap;

use crate::backend::Backend;
use crate::module::Module;

pub(crate) struct State<B> {
    backend: B,
    modules: FxHashMap<TypeId, &'static dyn Any>,
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

// FIXME: remove once upstream is stabilized.
#[inline]
unsafe fn downcast_ref_unchecked<T: Any>(value: &dyn Any) -> &T {
    debug_assert!(value.is::<T>());
    // SAFETY: caller guarantees that T is the correct type.
    unsafe { &*(value as *const dyn Any as *const T) }
}
