use std::any::TypeId;
use std::cell::RefCell;
use std::rc::Rc;

use crate::{runtime::Runtime, *};

pub struct Ctx<S: Plugin> {
    runtime: Rc<RefCell<Runtime>>,
    _marker: std::marker::PhantomData<S>,
}

impl<S: Plugin> Clone for Ctx<S> {
    fn clone(&self) -> Self {
        Self {
            runtime: Rc::clone(&self.runtime),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S: Plugin> Ctx<S> {
    pub fn new(runtime: Rc<RefCell<Runtime>>) -> Self {
        Self { runtime, _marker: std::marker::PhantomData }
    }

    /// TODO: docs
    ///
    /// # Panics
    ///
    /// Panics if the..
    #[track_caller]
    pub fn with_plugin<P, F, R>(&self, fun: F) -> R
    where
        P: Plugin,
        F: FnOnce(&P) -> R,
    {
        if TypeId::of::<S>() == TypeId::of::<P>() {
            panic!("Plugin cannot call itself")
        }

        self.runtime.borrow().with_plugin(fun)
    }
}
