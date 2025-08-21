use core::cell::RefCell;
use core::ops::{Deref, DerefMut};
use std::rc::Rc;

use editor::{AccessMut, AgentId, Shared};
use nohash::IntMap as NoHashMap;
use slotmap::SlotMap;
use smallvec::{SmallVec, smallvec_inline};

use crate::Neovim;
use crate::buffer::{BufferId, BuffersState, NeovimBuffer};
use crate::events::{self, AugroupId, CallbacksContainer, Event};
use crate::option::{SetUneditableEolAgentIds, UneditableEndOfLine};
use crate::oxi::api;

/// TODO: docs.
pub struct EventHandle {
    /// A list of `(callback_key, event_kind)` pairs, where the `callback_key`
    /// is the key of the callback stored in the [`Callbacks`]' [`SlotMap`].
    inner: SmallVec<[(slotmap::DefaultKey, EventKind); 1]>,
}

pub(crate) struct EventsBorrow<'a> {
    pub(crate) borrow: &'a mut Events,
    pub(crate) handle: Shared<Events>,
}

/// TODO: docs.
pub(crate) struct Events {
    /// TODO: docs.
    pub(crate) agent_ids: AgentIds,

    /// The ID of the group that `Self` will register autocommands in.
    pub(crate) augroup_id: AugroupId,

    /// TODO: docs.
    pub(crate) buffers_state: BuffersState,

    /// TODO: docs.
    pub(crate) on_uneditable_eol_set: Option<Callbacks<UneditableEndOfLine>>,

    /// The callback registered to the [`BufReadPost`] event, or `None` if no
    /// callback have been registered to that event.
    pub(crate) on_buffer_created: Option<Callbacks<events::BufReadPost>>,

    /// Map from a buffer's ID to the callbacks registered to the [`OnBytes`]
    /// event on that buffer.
    pub(crate) on_buffer_edited:
        NoHashMap<BufferId, Callbacks<events::OnBytes>>,

    /// The callback registered to the [`BufEnter`] event, or `None` if no
    /// callback have been registered to that event.
    pub(crate) on_buffer_focused: Option<Callbacks<events::BufEnter>>,

    /// Map from a buffer's ID to the callbacks registered to the [`BufUnload`]
    /// event on that buffer.
    pub(crate) on_buffer_removed:
        NoHashMap<BufferId, Callbacks<events::BufUnload>>,

    /// Map from a buffer's ID to the callbacks registered to the
    /// [`BufWritePost`] event on that buffer.
    pub(crate) on_buffer_saved:
        NoHashMap<BufferId, Callbacks<events::BufWritePost>>,

    /// Map from a buffer's ID to the callbacks registered to the [`BufLeave`]
    /// event on that buffer.
    pub(crate) on_buffer_unfocused:
        NoHashMap<BufferId, Callbacks<events::BufLeave>>,

    /// Map from a buffer's ID to the callbacks registered to the
    /// [`CursorMoved`] event on that buffer.
    pub(crate) on_cursor_moved:
        NoHashMap<BufferId, Callbacks<events::CursorMoved>>,

    /// The callback registered to the [`ModeChanged`] event, or `None` if no
    /// callback have been registered to that event .
    pub(crate) on_mode_changed: Option<Callbacks<events::ModeChanged>>,
}

/// TODO: docs.
#[derive(Default)]
pub(crate) struct AgentIds {
    /// TODO: docs.
    pub(crate) created_buffer: NoHashMap<BufferId, AgentId>,

    /// TODO: docs.
    pub(crate) edited_buffer: NoHashMap<BufferId, AgentId>,

    /// TODO: docs.
    pub(crate) focused_buffer: NoHashMap<BufferId, AgentId>,

    /// TODO: docs.
    pub(crate) moved_cursor: NoHashMap<BufferId, AgentId>,

    /// TODO: docs.
    pub(crate) removed_buffer: NoHashMap<BufferId, AgentId>,

    /// TODO: docs.
    pub(crate) saved_buffer: NoHashMap<BufferId, AgentId>,

    /// TODO: docs.
    pub(crate) set_uneditable_eol: SetUneditableEolAgentIds,
}

/// Groups the callbacks registered for a specific event type.
pub(crate) struct Callbacks<Ev: Event> {
    /// A map from callback key to the corresponding function.
    #[allow(clippy::type_complexity)]
    inner: SlotMap<slotmap::DefaultKey, Rc<dyn Fn(Ev::Args<'_>) + 'static>>,

    /// The value returned by [`register`](Event::register)ing the event.
    register_output: Ev::RegisterOutput,
}

pub(crate) enum EventKind {
    BufEnter(events::BufEnter),
    BufLeave(events::BufLeave),
    BufReadPost(events::BufReadPost),
    BufUnload(events::BufUnload),
    BufWritePost(events::BufWritePost),
    CursorMoved(events::CursorMoved),
    ModeChanged(events::ModeChanged),
    OnBytes(events::OnBytes),
    UneditableEolSet(UneditableEndOfLine),
}

impl EventHandle {
    /// Merges two [`EventHandle`]s into one.
    #[inline]
    pub(crate) fn merge(mut self, mut other: Self) -> Self {
        self.inner.extend(other.inner.drain(..));
        self
    }

    #[inline]
    fn new(
        event_key: slotmap::DefaultKey,
        event_kind: EventKind,
        _events: Shared<Events>,
    ) -> Self {
        Self { inner: smallvec_inline![(event_key, event_kind)] }
    }
}

impl<'a> EventsBorrow<'a> {
    #[inline]
    pub(crate) fn reborrow(&mut self) -> EventsBorrow<'_> {
        EventsBorrow { borrow: self.borrow, handle: self.handle.clone() }
    }
}

impl Events {
    /// Returns whether there's at least one callback registered for the
    /// given event.
    pub(crate) fn contains(&mut self, event: &impl Event) -> bool {
        event.container(self).get_mut(event.key()).is_some()
    }

    pub(crate) fn insert<T: Event>(
        events: Shared<Self>,
        event: T,
        fun: impl FnMut(T::Args<'_>) + 'static,
    ) -> EventHandle {
        let event_key = events.with_mut(|this| {
            if let Some(callbacks) = event.container(this).get_mut(event.key())
            {
                return callbacks.insert(fun);
            }

            let output = event.register(EventsBorrow {
                borrow: this,
                handle: events.clone(),
            });

            let mut callbacks = Callbacks::new(output);

            let event_key = callbacks.insert(fun);

            event.container(this).insert(event.key(), callbacks);

            event_key
        });

        let event_kind = event.kind();

        EventHandle::new(event_key, event_kind, events)
    }

    pub(crate) fn insert2<T: Event>(
        &mut self,
        event: T,
        callback: impl FnMut(T::Args<'_>) + 'static,
        nvim: impl AccessMut<Neovim> + 'static,
    ) -> EventHandle {
        let callback_key = if let Some(callbacks) =
            event.container(self).get_mut(event.key())
        {
            callbacks.insert(callback)
        } else {
            let register_output = event.register2(self, nvim);
            let mut callbacks = Callbacks::new(register_output);
            let callback_key = callbacks.insert(callback);
            event.container(self).insert(event.key(), callbacks);
            callback_key
        };

        EventHandle { inner: smallvec_inline![(callback_key, event.kind())] }
    }

    pub(crate) fn new(
        augroup_name: &str,
        buffers_state: BuffersState,
    ) -> Self {
        let augroup_id = api::create_augroup(
            augroup_name,
            &api::opts::CreateAugroupOpts::builder().clear(true).build(),
        )
        .expect("couldn't create augroup");

        Self {
            augroup_id,
            agent_ids: Default::default(),
            buffers_state,
            on_buffer_created: Default::default(),
            on_buffer_edited: Default::default(),
            on_buffer_focused: Default::default(),
            on_buffer_removed: Default::default(),
            on_buffer_saved: Default::default(),
            on_buffer_unfocused: Default::default(),
            on_cursor_moved: Default::default(),
            on_mode_changed: Default::default(),
            on_uneditable_eol_set: Default::default(),
        }
    }

    #[track_caller]
    pub(crate) fn buffer<'a>(
        buffer_id: BufferId,
        events: &'a Shared<Events>,
        bufs_state: &'a BuffersState,
    ) -> NeovimBuffer<'a> {
        NeovimBuffer::new(buffer_id, events, bufs_state)
            .expect("couldn't get buffer")
    }

    /// TODO: docs.
    pub(crate) fn cleanup_event_handle(&mut self, event_handle: EventHandle) {
        use EventKind::*;

        for (cb_key, event_kind) in event_handle.inner.into_iter() {
            match &event_kind {
                BufEnter(ev) => self.remove_callback(ev, cb_key),
                BufLeave(ev) => self.remove_callback(ev, cb_key),
                BufReadPost(ev) => self.remove_callback(ev, cb_key),
                BufUnload(ev) => self.remove_callback(ev, cb_key),
                BufWritePost(ev) => self.remove_callback(ev, cb_key),
                CursorMoved(ev) => self.remove_callback(ev, cb_key),
                ModeChanged(ev) => self.remove_callback(ev, cb_key),
                OnBytes(ev) => self.remove_callback(ev, cb_key),
                UneditableEolSet(ev) => self.remove_callback(ev, cb_key),
            }
        }
    }

    /// Removes the callback registered for the given event with the given
    /// key.
    ///
    /// If the callback was the last one on the event, the event itself will be
    /// unregistered.
    fn remove_callback<Ev: Event>(
        &mut self,
        event: &Ev,
        callback_key: slotmap::DefaultKey,
    ) {
        let mut container = event.container(self);
        let Some(callbacks) = container.get_mut(event.key()) else { return };
        callbacks.remove(callback_key);
        if callbacks.is_empty() {
            match container.remove(event.key()) {
                Some(callbacks) => Ev::unregister(callbacks.register_output),
                None => unreachable!("just checked"),
            }
        }
    }
}

impl<T: Event> Callbacks<T> {
    #[allow(clippy::type_complexity)]
    #[inline]
    pub(crate) fn cloned(
        &self,
    ) -> impl IntoIterator<Item = Rc<dyn Fn(T::Args<'_>)>> + use<T> {
        self.inner.values().map(Rc::clone).collect::<SmallVec<[_; 2]>>()
    }

    #[inline]
    fn insert(
        &mut self,
        fun: impl FnMut(T::Args<'_>) + 'static,
    ) -> slotmap::DefaultKey {
        let fun = RefCell::new(fun);

        self.inner.insert(Rc::new(move |args| {
            fun.borrow_mut()(args);
        }))
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[inline]
    fn new(output: T::RegisterOutput) -> Self {
        Self { inner: Default::default(), register_output: output }
    }

    #[inline]
    fn remove(&mut self, callback_key: slotmap::DefaultKey) {
        self.inner.remove(callback_key);
    }
}

impl Deref for EventsBorrow<'_> {
    type Target = Events;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.borrow
    }
}

impl DerefMut for EventsBorrow<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.borrow
    }
}
