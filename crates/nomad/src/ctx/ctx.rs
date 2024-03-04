use alloc::rc::Rc;
use core::cell::RefCell;
use core::mem;
use core::ops::Deref;

use pond::Engine;

use super::{Get, Set};

/// TODO: docs
#[derive(Clone, Default)]
pub(crate) struct Ctx {
    ctx: Rc<RefCell<CtxInner>>,
}

impl Ctx {
    /// TODO: docs
    #[inline]
    pub(crate) fn with_init<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&InitCtx) -> R,
    {
        let ctx = self.ctx.borrow();
        f(ctx.as_init())
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn with_set<'this, F, R>(&'this self, f: F) -> R
    where
        F: FnOnce(&mut SetCtx) -> R,
        R: 'this,
    {
        let mut ctx = self.ctx.borrow_mut();
        f(ctx.as_set())
    }
}

/// TODO: docs
#[derive(Default)]
pub(crate) struct CtxInner {
    engine: Engine,
}

impl CtxInner {
    /// TODO: docs
    #[inline]
    pub fn as_init(&self) -> &InitCtx {
        // SAFETY: `InitCtx` and `Ctx` have the same layout.
        unsafe { mem::transmute(self) }
    }

    /// TODO: docs
    #[inline]
    pub fn as_set(&mut self) -> &mut SetCtx {
        // SAFETY: `SetCtx` and `Ctx` have the same layout.
        unsafe { mem::transmute(self) }
    }
}

/// TODO: docs
pub struct InitCtx {
    ctx: CtxInner,
}

impl InitCtx {
    /// TODO: docs
    #[inline]
    pub fn new_input<T>(&self, input: T) -> (Get<T>, Set<T>) {
        let (get, set) = self.ctx.engine.var(input);
        (Get::new(get), Set::new(set))
    }
}

/// TODO: docs
pub struct GetCtx {
    ctx: CtxInner,
}

impl GetCtx {
    #[inline]
    pub(super) fn as_engine(&self) -> &Engine {
        &self.ctx.engine
    }
}

/// TODO: docs
pub struct SetCtx {
    ctx: CtxInner,
}

impl SetCtx {
    #[inline]
    pub(super) fn as_engine_mut(&mut self) -> &mut Engine {
        &mut self.ctx.engine
    }
}

impl Deref for SetCtx {
    type Target = GetCtx;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // SAFETY: `SetCtx` and `GetCtx` have the same layout.
        unsafe { mem::transmute(&self.ctx) }
    }
}
