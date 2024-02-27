use core::mem;
use core::ops::Deref;

use pond::Engine;

use crate::{Get, Set};

/// TODO: docs
#[derive(Default)]
pub struct Ctx {
    engine: Engine,
}

impl Ctx {
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

    /// TODO: docs
    #[inline]
    pub fn new_input<T>(&self, input: T) -> (Get<T>, Set<T>) {
        self.as_init().new_input(input)
    }
}

/// TODO: docs
pub struct InitCtx {
    ctx: Ctx,
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
    ctx: Ctx,
}

/// TODO: docs
pub struct SetCtx {
    ctx: Ctx,
}

impl Deref for SetCtx {
    type Target = GetCtx;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // SAFETY: `SetCtx` and `GetCtx` have the same layout.
        unsafe { mem::transmute(&self.ctx) }
    }
}
