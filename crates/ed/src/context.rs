// State{Handle,Mut} is used in:
//
// - api_ctx;
// - command_builder;
// - plugin;

use core::marker::PhantomData;

use crate::notify::{Name, Namespace};
use crate::plugin::PluginId;
use crate::state::State;
use crate::{Backend, Shared};

/// TODO: docs.
pub trait BorrowState {
    #[doc(hidden)]
    type Borrow<Ed: Backend>: Borrow<Ed>;
}

/// TODO: docs.
pub struct Context<Ed: Backend, B: BorrowState = NotBorrowed> {
    borrow: B::Borrow<Ed>,
}

/// TODO: docs.
pub struct NotBorrowed;

/// TODO: docs.
pub struct Borrowed<'a> {
    _lifetime: PhantomData<&'a ()>,
}

impl BorrowState for NotBorrowed {
    type Borrow<Ed: Backend> = NotBorrowedInner<Ed>;
}

impl<'a> BorrowState for Borrowed<'a> {
    type Borrow<Ed: Backend> = BorrowedInner<'a, Ed>;
}

/// TODO: docs.
#[doc(hidden)]
pub trait Borrow<Ed: Backend> {
    /// TODO: docs.
    fn namespace(&self) -> &Namespace;

    /// TODO: docs.
    fn plugin_id(&self) -> PluginId;

    /// TODO: docs.
    fn with_state<T>(&mut self, f: impl FnOnce(&mut State<Ed>) -> T) -> T;
}

/// TODO: docs.
#[doc(hidden)]
pub struct NotBorrowedInner<Ed: Backend> {
    namespace: Namespace,
    plugin_id: PluginId,
    state_handle: Shared<State<Ed>>,
}

/// TODO: docs.
#[doc(hidden)]
pub struct BorrowedInner<'a, Ed: Backend> {
    namespace: &'a Namespace,
    plugin_id: PluginId,
    state_handle: Shared<State<Ed>>,
    state_mut: &'a mut State<Ed>,
}

impl<Ed: Backend> Borrow<Ed> for NotBorrowedInner<Ed> {
    #[inline]
    fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    #[inline]
    fn plugin_id(&self) -> PluginId {
        self.plugin_id
    }

    #[inline]
    fn with_state<T>(&mut self, f: impl FnOnce(&mut State<Ed>) -> T) -> T {
        self.state_handle.with_mut(f)
    }
}

impl<Ed: Backend> Borrow<Ed> for BorrowedInner<'_, Ed> {
    #[inline]
    fn namespace(&self) -> &Namespace {
        self.namespace
    }

    #[inline]
    fn plugin_id(&self) -> PluginId {
        self.plugin_id
    }

    #[inline]
    fn with_state<T>(&mut self, f: impl FnOnce(&mut State<Ed>) -> T) -> T {
        f(self.state_mut)
    }
}
