// State{Handle,Mut} is used in:
//
// - api_ctx;
// - command_builder;
// - plugin;

use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use crate::notify::{Name, Namespace};
use crate::plugin::{Plugin, PluginId};
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
    state_handle: &'a Shared<State<Ed>>,
    state: &'a mut State<Ed>,
}

impl<Ed: Backend, B: BorrowState> Context<Ed, B> {
    #[inline]
    fn namespace(&self) -> &Namespace {
        self.borrow.namespace()
    }

    #[inline]
    fn plugin_id(&self) -> PluginId {
        self.borrow.plugin_id()
    }
}

impl<Ed: Backend> Context<Ed, NotBorrowed> {
    /// TODO: docs.
    #[inline]
    pub fn with_mut<T>(
        &self,
        fun: impl FnOnce(&mut Context<Ed, Borrowed<'_>>) -> T,
    ) -> T {
        self.borrow.state_handle.with_mut(|state| {
            let mut ctx = Context {
                borrow: BorrowedInner {
                    namespace: self.namespace(),
                    plugin_id: self.plugin_id(),
                    state_handle: &self.borrow.state_handle,
                    state,
                },
            };
            fun(&mut ctx)
        })
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn from_editor(editor: Ed) -> Self {
        Self {
            borrow: NotBorrowedInner {
                namespace: Namespace::default(),
                plugin_id: <crate::state::ResumeUnwinding as Plugin<Ed>>::id(),
                state_handle: Shared::new(State::new(editor)),
            },
        }
    }
}

impl<Ed: Backend, B: BorrowState> Deref for Context<Ed, B>
where
    B::Borrow<Ed>: Deref<Target = Ed>,
{
    type Target = Ed;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.borrow.deref()
    }
}

impl<Ed: Backend, B: BorrowState> DerefMut for Context<Ed, B>
where
    B::Borrow<Ed>: DerefMut<Target = Ed>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.borrow.deref_mut()
    }
}

impl BorrowState for NotBorrowed {
    type Borrow<Ed: Backend> = NotBorrowedInner<Ed>;
}

impl<'a> BorrowState for Borrowed<'a> {
    type Borrow<Ed: Backend> = BorrowedInner<'a, Ed>;
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
        f(self.state)
    }
}

impl<Ed: Backend> Deref for BorrowedInner<'_, Ed> {
    type Target = Ed;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.state.deref()
    }
}

impl<Ed: Backend> DerefMut for BorrowedInner<'_, Ed> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state.deref_mut()
    }
}
