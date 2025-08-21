use core::hash::{BuildHasher, Hash};
use std::collections::HashMap;

use editor::AccessMut;

use crate::Neovim;
use crate::events::{Callbacks, EventKind, Events, EventsBorrow};

/// TODO: docs.
pub(crate) trait Event: Sized {
    /// The type of arguments given to the callbacks registered for this
    /// event.
    type Args<'a>;

    /// TODO: docs.
    type Container<'ev>: CallbacksContainer<Self>;

    /// The output of [`register()`](Event::register)ing the event.
    type RegisterOutput;

    /// TODO: docs.
    fn container<'ev>(&self, event: &'ev mut Events) -> Self::Container<'ev>;

    /// TODO: docs.
    fn key(&self) -> <Self::Container<'_> as CallbacksContainer<Self>>::Key;

    /// TODO: docs.
    fn kind(&self) -> EventKind;

    /// TODO: docs.
    fn register(&self, events: EventsBorrow) -> Self::RegisterOutput;

    /// TODO: docs.
    fn register2(
        &self,
        _events: &mut Events,
        _nvim: impl AccessMut<Neovim> + 'static,
    ) -> Self::RegisterOutput {
        todo!();
    }

    /// TODO: docs.
    fn unregister(out: Self::RegisterOutput);
}

/// TODO: docs.
pub(crate) trait CallbacksContainer<Ev: Event> {
    /// TODO: docs.
    type Key;

    /// TODO: docs.
    fn get_mut(&mut self, key: Self::Key) -> Option<&mut Callbacks<Ev>>;

    /// TODO: docs.
    fn insert(&mut self, key: Self::Key, callbacks: Callbacks<Ev>);

    /// TODO: docs.
    fn remove(&mut self, key: Self::Key) -> Option<Callbacks<Ev>>;
}

impl<Ev: Event> CallbacksContainer<Ev> for Option<Callbacks<Ev>> {
    type Key = ();

    #[inline]
    fn get_mut(&mut self, _: ()) -> Option<&mut Callbacks<Ev>> {
        self.as_mut()
    }

    #[inline]
    fn insert(&mut self, _: (), callbacks: Callbacks<Ev>) {
        *self = Some(callbacks);
    }

    #[track_caller]
    #[inline]
    fn remove(&mut self, _: ()) -> Option<Callbacks<Ev>> {
        self.take()
    }
}

impl<Ev, Key, Hasher> CallbacksContainer<Ev>
    for HashMap<Key, Callbacks<Ev>, Hasher>
where
    Ev: Event,
    Key: Eq + Hash,
    Hasher: BuildHasher,
{
    type Key = Key;

    #[inline]
    fn get_mut(&mut self, key: Key) -> Option<&mut Callbacks<Ev>> {
        Self::get_mut(self, &key)
    }

    #[inline]
    fn insert(&mut self, key: Key, callbacks: Callbacks<Ev>) {
        Self::insert(self, key, callbacks);
    }

    #[track_caller]
    #[inline]
    fn remove(&mut self, key: Key) -> Option<Callbacks<Ev>> {
        Self::remove(self, &key)
    }
}

impl<Ev: Event, T: CallbacksContainer<Ev>> CallbacksContainer<Ev> for &mut T {
    type Key = T::Key;

    #[inline]
    fn get_mut(&mut self, key: Self::Key) -> Option<&mut Callbacks<Ev>> {
        CallbacksContainer::get_mut(*self, key)
    }

    #[inline]
    fn insert(&mut self, key: Self::Key, callbacks: Callbacks<Ev>) {
        CallbacksContainer::insert(*self, key, callbacks);
    }

    #[inline]
    fn remove(&mut self, key: Self::Key) -> Option<Callbacks<Ev>> {
        CallbacksContainer::remove(*self, key)
    }
}
