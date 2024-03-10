use core::cell::{OnceCell, UnsafeCell};
use core::mem;

use pond::Engine;

use super::{Get, Set};

thread_local! {
    static CTX: Ctx = const { Ctx::new() };
}

/// TODO: docs
#[inline]
pub fn input<T>(input: T) -> (Get<T>, Set<T>) {
    CTX.with(|ctx| ctx.inner().input(input))
}

/// TODO: docs
#[inline]
pub(crate) fn init() {
    CTX.with(Ctx::init);
}

/// TODO: docs
#[inline]
pub(super) fn get<T>(get: &Get<T>) -> &T {
    CTX.with(|ctx| ctx.inner().get(get))
}

/// TODO: docs
#[inline]
pub(super) fn set<T>(set: &Set<T>, new_value: T) {
    CTX.with(|ctx| ctx.inner().set(set, new_value));
}

/// TODO: docs
#[inline]
pub(super) fn update<T, F>(set: &Set<T>, update_with: F)
where
    F: FnOnce(&mut T),
{
    CTX.with(|ctx| ctx.inner().update(set, update_with));
}

/// TODO: docs
struct Ctx {
    inner: OnceCell<CtxInner>,
}

impl Ctx {
    /// TODO: docs
    #[inline]
    fn inner(&self) -> &CtxInner {
        match self.inner.get() {
            Some(inner) => inner,
            None => panic!("tried to access the Ctx from another thread"),
        }
    }

    #[inline]
    fn init(&self) {
        if self.inner.set(CtxInner::default()).is_err() {
            panic!("tried to initialize the Ctx more than once")
        }
    }

    /// TODO: docs
    const fn new() -> Self {
        Self { inner: OnceCell::new() }
    }
}

#[derive(Default)]
struct CtxInner {
    engine: UnsafeCell<Engine>,
}

impl CtxInner {
    /// TODO: docs
    #[inline]
    fn engine(&self) -> &Engine {
        self.engine_mut()
    }

    /// TODO: docs
    #[allow(clippy::mut_from_ref)]
    fn engine_mut(&self) -> &mut Engine {
        unsafe { &mut *(self.engine.get()) }
    }

    #[inline]
    fn get<'get, T>(&self, get: &'get Get<T>) -> &'get T {
        let out = get.inner().get(self.engine());
        unsafe { mem::transmute::<&T, &'get T>(out) }
    }

    #[inline]
    fn input<T>(&self, value: T) -> (Get<T>, Set<T>) {
        let (get, set) = self.engine().var(value);
        (Get::new(get), Set::new(set))
    }

    #[inline]
    fn set<T>(&self, set: &Set<T>, new_value: T) {
        set.inner().set(new_value, self.engine_mut());
    }

    #[inline]
    fn update<T, F>(&self, set: &Set<T>, update_with: F)
    where
        F: FnOnce(&mut T),
    {
        set.inner().update(update_with, self.engine_mut());
    }
}
