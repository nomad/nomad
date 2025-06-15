use core::cell::RefCell;
use core::ops::{Deref, DerefMut};
use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};
use std::rc::Rc;

use ed::{AgentId, Edit, Shared};
use nohash::IntMap as NoHashMap;
use slotmap::SlotMap;
use smallvec::{SmallVec, smallvec_inline};

use crate::buffer::{BufferId, BuffersState, NeovimBuffer};
use crate::cursor::NeovimCursor;
use crate::mode::ModeStr;
use crate::option::{SetUneditableEolAgentIds, UneditableEndOfLine};
use crate::oxi::api;

pub(crate) type AugroupId = u32;
pub(crate) type AutocmdId = u32;

/// TODO: docs.
pub struct EventHandle {
    events: Shared<Events>,
    /// A list of `(callback_key, event_kind)` pairs, where the `callback_key`
    /// is the key of the callback stored in the [`Callbacks`]' [`SlotMap`].
    event_keys_kind: SmallVec<[(slotmap::DefaultKey, EventKind); 1]>,
}

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
    fn unregister(out: Self::RegisterOutput);

    /// TODO: docs.
    #[inline]
    fn cleanup(&self, event_key: slotmap::DefaultKey, events: &mut Events) {
        let mut container = self.container(events);
        let Some(callbacks) = container.get_mut(self.key()) else { return };
        callbacks.remove(event_key);
        if callbacks.is_empty() {
            match container.remove(self.key()) {
                Some(callbacks) => Self::unregister(callbacks.output),
                None => unreachable!("just checked"),
            }
        }
    }
}

pub(crate) struct EventsBorrow<'a> {
    pub(crate) borrow: &'a mut Events,
    pub(crate) handle: Shared<Events>,
}

pub(crate) struct Events {
    pub(crate) agent_ids: AgentIds,
    pub(crate) augroup_id: AugroupId,
    pub(crate) buffers_state: BuffersState,
    pub(crate) on_uneditable_eol_set: Option<Callbacks<UneditableEndOfLine>>,
    on_buffer_created: Option<Callbacks<BufReadPost>>,
    on_buffer_edited: NoHashMap<BufferId, Callbacks<OnBytes>>,
    on_buffer_focused: Option<Callbacks<BufEnter>>,
    on_buffer_removed: NoHashMap<BufferId, Callbacks<BufUnload>>,
    on_buffer_saved: NoHashMap<BufferId, Callbacks<BufWritePost>>,
    on_buffer_unfocused: NoHashMap<BufferId, Callbacks<BufLeave>>,
    on_cursor_moved: NoHashMap<BufferId, Callbacks<CursorMoved>>,
    on_mode_changed: Option<Callbacks<ModeChanged>>,
}

#[derive(Default)]
pub(crate) struct AgentIds {
    pub(crate) created_buffer: NoHashMap<BufferId, AgentId>,
    pub(crate) edited_buffer: NoHashMap<BufferId, AgentId>,
    pub(crate) focused_buffer: NoHashMap<BufferId, AgentId>,
    pub(crate) moved_cursor: NoHashMap<BufferId, AgentId>,
    pub(crate) removed_buffer: NoHashMap<BufferId, AgentId>,
    pub(crate) saved_buffer: NoHashMap<BufferId, AgentId>,
    pub(crate) set_uneditable_eol: SetUneditableEolAgentIds,
}

pub(crate) struct Callbacks<T: Event> {
    #[allow(clippy::type_complexity)]
    inner: SlotMap<slotmap::DefaultKey, Rc<dyn Fn(T::Args<'_>) + 'static>>,
    output: T::RegisterOutput,
}

#[derive(Clone, Copy)]
pub(crate) struct BufEnter;

#[derive(Clone, Copy)]
pub(crate) struct BufLeave(pub(crate) BufferId);

#[derive(Clone, Copy)]
pub(crate) struct BufReadPost;

#[derive(Clone, Copy)]
pub(crate) struct BufUnload(pub(crate) BufferId);

#[derive(Clone, Copy)]
pub(crate) struct BufWritePost(pub(crate) BufferId);

#[derive(Clone, Copy)]
pub(crate) struct CursorMoved(pub(crate) BufferId);

#[derive(Clone, Copy)]
pub(crate) struct ModeChanged;

#[derive(Clone, Copy)]
pub(crate) struct OnBytes(pub(crate) BufferId);

pub(crate) enum EventKind {
    BufEnter(BufEnter),
    BufLeave(BufLeave),
    BufReadPost(BufReadPost),
    BufUnload(BufUnload),
    BufWritePost(BufWritePost),
    CursorMoved(CursorMoved),
    ModeChanged(ModeChanged),
    OnBytes(OnBytes),
    UneditableEolSet(UneditableEndOfLine),
}

impl EventHandle {
    /// Merges two [`EventHandle`]s into one.
    #[inline]
    pub(crate) fn merge(mut self, mut other: Self) -> Self {
        self.event_keys_kind.extend(other.event_keys_kind.drain(..));
        self
    }

    #[inline]
    fn new(
        event_key: slotmap::DefaultKey,
        event_kind: EventKind,
        events: Shared<Events>,
    ) -> Self {
        Self {
            events,
            event_keys_kind: smallvec_inline![(event_key, event_kind)],
        }
    }
}

impl<'a> EventsBorrow<'a> {
    #[inline]
    pub(crate) fn reborrow(&mut self) -> EventsBorrow<'_> {
        EventsBorrow { borrow: self.borrow, handle: self.handle.clone() }
    }
}

impl Events {
    pub(crate) fn contains(&mut self, event: &impl Event) -> bool {
        event.container(self).get_mut(event.key()).is_some()
    }

    pub(crate) fn insert<T: Event>(
        events: Shared<Self>,
        event: T,
        fun: impl FnMut(T::Args<'_>) + 'static,
    ) -> EventHandle {
        let event_kind = event.kind();

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

        EventHandle::new(event_key, event_kind, events)
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

    pub(crate) fn buffer<'a>(
        buffer_id: BufferId,
        events: &'a Shared<Events>,
        bufs_state: &'a BuffersState,
    ) -> NeovimBuffer<'a> {
        NeovimBuffer::new(buffer_id, events, bufs_state)
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
        Self { inner: Default::default(), output }
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

impl Event for BufEnter {
    type Args<'a> = (&'a NeovimBuffer<'a>, AgentId);
    type Container<'ev> = &'ev mut Option<Callbacks<Self>>;
    type RegisterOutput = AutocmdId;

    #[inline]
    fn container<'ev>(&self, events: &'ev mut Events) -> Self::Container<'ev> {
        &mut events.on_buffer_focused
    }

    #[inline]
    fn kind(&self) -> EventKind {
        EventKind::BufEnter(*self)
    }

    #[inline]
    fn key(&self) {}

    #[inline]
    fn register(&self, events: EventsBorrow) -> AutocmdId {
        let augroup_id = events.augroup_id;

        let bufs_state = events.borrow.buffers_state.clone();
        let events = events.handle;

        let opts = api::opts::CreateAutocmdOpts::builder()
            .group(augroup_id)
            .callback(move |args: api::types::AutocmdCallbackArgs| {
                let buffer_id = BufferId::new(args.buffer);

                let Some((callbacks, focused_by)) = events.with_mut(|ev| {
                    let callbacks = ev.on_buffer_focused.as_ref()?;

                    let focused_by = ev
                        .agent_ids
                        .focused_buffer
                        .remove(&buffer_id)
                        .unwrap_or(AgentId::UNKNOWN);

                    Some((callbacks.cloned(), focused_by))
                }) else {
                    return true;
                };

                let buffer = Events::buffer(buffer_id, &events, &bufs_state);

                for callback in callbacks {
                    callback((&buffer, focused_by));
                }

                false
            })
            .build();

        api::create_autocmd(["BufEnter"], &opts)
            .expect("couldn't create autocmd on BufEnter")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}

impl Event for BufLeave {
    type Args<'a> = (&'a NeovimBuffer<'a>, AgentId);
    type Container<'ev> = &'ev mut NoHashMap<BufferId, Callbacks<Self>>;
    type RegisterOutput = AutocmdId;

    #[inline]
    fn container<'ev>(&self, events: &'ev mut Events) -> Self::Container<'ev> {
        &mut events.on_buffer_unfocused
    }

    #[inline]
    fn key(&self) -> BufferId {
        self.0
    }

    #[inline]
    fn kind(&self) -> EventKind {
        EventKind::BufLeave(*self)
    }

    #[inline]
    fn register(&self, events: EventsBorrow) -> AutocmdId {
        let augroup_id = events.augroup_id;

        let bufs_state = events.borrow.buffers_state.clone();
        let events = events.handle;

        let opts = api::opts::CreateAutocmdOpts::builder()
            .group(augroup_id)
            .buffer(self.0.into())
            .callback(move |args: api::types::AutocmdCallbackArgs| {
                let buffer_id = BufferId::new(args.buffer);

                let Some((callbacks, removed_by)) = events.with_mut(|ev| {
                    let callbacks = ev.on_buffer_removed.get(&buffer_id)?;
                    Some((callbacks.cloned(), AgentId::UNKNOWN))
                }) else {
                    return true;
                };

                let buffer = Events::buffer(buffer_id, &events, &bufs_state);

                for callback in callbacks {
                    callback((&buffer, removed_by));
                }

                false
            })
            .build();

        api::create_autocmd(["BufLeave"], &opts)
            .expect("couldn't create autocmd on BufLeave")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}

impl Event for BufReadPost {
    type Args<'a> = (&'a NeovimBuffer<'a>, AgentId);
    type Container<'ev> = &'ev mut Option<Callbacks<Self>>;
    type RegisterOutput = AutocmdId;

    #[inline]
    fn container<'ev>(&self, events: &'ev mut Events) -> Self::Container<'ev> {
        &mut events.on_buffer_created
    }

    #[inline]
    fn key(&self) {}

    #[inline]
    fn kind(&self) -> EventKind {
        EventKind::BufReadPost(*self)
    }

    #[inline]
    fn register(&self, events: EventsBorrow) -> AutocmdId {
        let augroup_id = events.augroup_id;

        let bufs_state = events.borrow.buffers_state.clone();
        let events = events.handle;

        let opts = api::opts::CreateAutocmdOpts::builder()
            .group(augroup_id)
            .callback(move |args: api::types::AutocmdCallbackArgs| {
                let buffer_id = BufferId::new(args.buffer);

                let Some((callbacks, created_by)) = events.with_mut(|ev| {
                    let callbacks = ev.on_buffer_created.as_ref()?;

                    let created_by = ev
                        .agent_ids
                        .created_buffer
                        .remove(&buffer_id)
                        .unwrap_or(AgentId::UNKNOWN);

                    Some((callbacks.cloned(), created_by))
                }) else {
                    return true;
                };

                let buffer = Events::buffer(buffer_id, &events, &bufs_state);

                for callback in callbacks {
                    callback((&buffer, created_by));
                }

                false
            })
            .build();

        api::create_autocmd(["BufReadPost"], &opts)
            .expect("couldn't create autocmd on BufReadPost")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}

impl Event for BufUnload {
    type Args<'a> = (&'a NeovimBuffer<'a>, AgentId);
    type Container<'ev> = &'ev mut NoHashMap<BufferId, Callbacks<Self>>;
    type RegisterOutput = AutocmdId;

    #[inline]
    fn container<'ev>(&self, events: &'ev mut Events) -> Self::Container<'ev> {
        &mut events.on_buffer_removed
    }

    #[inline]
    fn key(&self) -> BufferId {
        self.0
    }

    #[inline]
    fn kind(&self) -> EventKind {
        EventKind::BufUnload(*self)
    }

    #[inline]
    fn register(&self, events: EventsBorrow) -> AutocmdId {
        let augroup_id = events.augroup_id;

        let bufs_state = events.borrow.buffers_state.clone();
        let events = events.handle;

        let opts = api::opts::CreateAutocmdOpts::builder()
            .group(augroup_id)
            .buffer(self.0.into())
            .callback(move |args: api::types::AutocmdCallbackArgs| {
                let buffer_id = BufferId::new(args.buffer);

                let Some((callbacks, removed_by)) = events.with_mut(|ev| {
                    let callbacks = ev.on_buffer_removed.get(&buffer_id)?;

                    let removed_by = ev
                        .agent_ids
                        .removed_buffer
                        .remove(&buffer_id)
                        .unwrap_or(AgentId::UNKNOWN);

                    Some((callbacks.cloned(), removed_by))
                }) else {
                    return true;
                };

                let buffer = Events::buffer(buffer_id, &events, &bufs_state);

                for callback in callbacks {
                    callback((&buffer, removed_by));
                }

                false
            })
            .build();

        api::create_autocmd(["BufUnload"], &opts)
            .expect("couldn't create autocmd on BufUnload")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}

impl Event for BufWritePost {
    type Args<'a> = (&'a NeovimBuffer<'a>, AgentId);
    type Container<'ev> = &'ev mut NoHashMap<BufferId, Callbacks<Self>>;
    type RegisterOutput = AutocmdId;

    #[inline]
    fn container<'ev>(&self, events: &'ev mut Events) -> Self::Container<'ev> {
        &mut events.on_buffer_saved
    }

    #[inline]
    fn key(&self) -> BufferId {
        self.0
    }

    #[inline]
    fn kind(&self) -> EventKind {
        EventKind::BufWritePost(*self)
    }

    #[inline]
    fn register(&self, events: EventsBorrow) -> AutocmdId {
        let augroup_id = events.augroup_id;

        let bufs_state = events.borrow.buffers_state.clone();
        let events = events.handle;

        let opts = api::opts::CreateAutocmdOpts::builder()
            .group(augroup_id)
            .buffer(self.0.into())
            .callback(move |args: api::types::AutocmdCallbackArgs| {
                let buffer_id = BufferId::new(args.buffer);

                let Some((callbacks, saved_by)) = events.with_mut(|ev| {
                    let callbacks = ev.on_buffer_saved.get(&buffer_id)?;

                    let saved_by = ev
                        .agent_ids
                        .saved_buffer
                        .remove(&buffer_id)
                        .unwrap_or(AgentId::UNKNOWN);

                    Some((callbacks.cloned(), saved_by))
                }) else {
                    return true;
                };

                let buffer = Events::buffer(buffer_id, &events, &bufs_state);

                for callback in callbacks {
                    callback((&buffer, saved_by));
                }

                false
            })
            .build();

        api::create_autocmd(["BufWritePost"], &opts)
            .expect("couldn't create autocmd on BufWritePost")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}

impl Event for CursorMoved {
    type Args<'a> = (&'a NeovimCursor<'a>, AgentId);
    type Container<'ev> = &'ev mut NoHashMap<BufferId, Callbacks<Self>>;
    type RegisterOutput = AutocmdId;

    #[inline]
    fn container<'ev>(&self, events: &'ev mut Events) -> Self::Container<'ev> {
        &mut events.on_cursor_moved
    }

    #[inline]
    fn key(&self) -> BufferId {
        self.0
    }

    #[inline]
    fn kind(&self) -> EventKind {
        EventKind::CursorMoved(*self)
    }

    #[inline]
    fn register(&self, events: EventsBorrow) -> Self::RegisterOutput {
        let augroup_id = events.augroup_id;

        let bufs_state = events.borrow.buffers_state.clone();
        let events = events.handle;

        let opts = api::opts::CreateAutocmdOpts::builder()
            .group(augroup_id)
            .buffer(self.0.into())
            .callback(move |args: api::types::AutocmdCallbackArgs| {
                let buffer_id = BufferId::new(args.buffer);

                let Some((callbacks, moved_by)) = events.with_mut(|ev| {
                    let callbacks = ev.on_cursor_moved.get(&buffer_id)?;

                    let moved_by = ev
                        .agent_ids
                        .moved_cursor
                        .remove(&buffer_id)
                        .unwrap_or(AgentId::UNKNOWN);

                    Some((callbacks.cloned(), moved_by))
                }) else {
                    return true;
                };

                let cursor = NeovimCursor::new(Events::buffer(
                    buffer_id,
                    &events,
                    &bufs_state,
                ));

                for callback in callbacks {
                    callback((&cursor, moved_by));
                }

                false
            })
            .build();

        // Neovim has 3 separate cursor-move-related autocommand events --
        // CursorMoved, CursorMovedI and CursorMovedC -- which are triggered
        // when the cursor is moved in Normal/Visual mode, Insert mode and in
        // the command line, respectively.
        //
        // Since ed has no concept of modes, we register the callback on both
        // CursorMoved and CursorMovedI.

        api::create_autocmd(["CursorMoved", "CursorMovedI"], &opts)
            .expect("couldn't create autocmd on CursorMoved{I}")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}

impl Event for ModeChanged {
    type Args<'a> = (NeovimBuffer<'a>, ModeStr<'a>, ModeStr<'a>, AgentId);
    type Container<'ev> = &'ev mut Option<Callbacks<Self>>;
    type RegisterOutput = AutocmdId;

    #[inline]
    fn container<'ev>(&self, events: &'ev mut Events) -> Self::Container<'ev> {
        &mut events.on_mode_changed
    }

    #[inline]
    fn key(&self) {}

    #[inline]
    fn kind(&self) -> EventKind {
        EventKind::ModeChanged(*self)
    }

    #[inline]
    fn register(&self, events: EventsBorrow) -> Self::RegisterOutput {
        let augroup_id = events.augroup_id;

        let bufs_state = events.borrow.buffers_state.clone();
        let events = events.handle;

        let opts = api::opts::CreateAutocmdOpts::builder()
            .group(augroup_id)
            .callback(move |args: api::types::AutocmdCallbackArgs| {
                let buffer_id = BufferId::new(args.buffer);

                let Some(callbacks) = events.with(|ev| {
                    ev.on_mode_changed.as_ref().map(Callbacks::cloned)
                }) else {
                    return true;
                };

                let (old_mode, new_mode) =
                    args.r#match.split_once(':').expect(
                        "expected a string with format \
                         \"{{old_mode}}:{{new_mode}}\"",
                    );

                let buffer = Events::buffer(buffer_id, &events, &bufs_state);
                let old_mode = ModeStr::new(old_mode);
                let new_mode = ModeStr::new(new_mode);

                for callback in callbacks {
                    callback((buffer, old_mode, new_mode, AgentId::UNKNOWN));
                }

                false
            })
            .build();

        api::create_autocmd(["ModeChanged"], &opts)
            .expect("couldn't create autocmd on ModeChanged")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}

impl Event for OnBytes {
    type Args<'a> = (&'a NeovimBuffer<'a>, &'a Edit);
    type Container<'ev> = &'ev mut NoHashMap<BufferId, Callbacks<Self>>;
    type RegisterOutput = ();

    #[inline]
    fn container<'ev>(&self, events: &'ev mut Events) -> Self::Container<'ev> {
        &mut events.on_buffer_edited
    }

    #[inline]
    fn key(&self) -> BufferId {
        self.0
    }

    #[inline]
    fn kind(&self) -> EventKind {
        EventKind::OnBytes(*self)
    }

    #[inline]
    fn register(&self, events: EventsBorrow) {
        let buffer_id = self.0;

        let bufs_state = events.borrow.buffers_state.clone();
        let events = events.handle;

        let opts = api::opts::BufAttachOpts::builder()
            .on_bytes(move |args: api::opts::OnBytesArgs| {
                let Some((callbacks, edited_by)) = events.with_mut(|ev| {
                    let callbacks = ev.on_buffer_edited.get(&buffer_id)?;

                    let edited_by = ev
                        .agent_ids
                        .edited_buffer
                        .remove(&buffer_id)
                        .unwrap_or(AgentId::UNKNOWN);

                    Some((callbacks.cloned(), edited_by))
                }) else {
                    return true;
                };

                let buffer = Events::buffer(buffer_id, &events, &bufs_state);

                let edit = Edit {
                    made_by: edited_by,
                    replacements: smallvec_inline![
                        buffer.replacement_of_on_bytes(args)
                    ],
                };

                for callback in callbacks {
                    callback((&buffer, &edit));
                }

                false
            })
            .build();

        api::Buffer::from(buffer_id)
            .attach(false, &opts)
            .expect("couldn't attach to buffer");
    }

    #[inline]
    fn unregister((): Self::RegisterOutput) {}
}

impl Drop for EventHandle {
    #[inline]
    fn drop(&mut self) {
        self.events.with_mut(|events| {
            for (key, kind) in self.event_keys_kind.drain(..) {
                match kind {
                    EventKind::BufEnter(ev) => ev.cleanup(key, events),
                    EventKind::BufLeave(ev) => ev.cleanup(key, events),
                    EventKind::BufReadPost(ev) => ev.cleanup(key, events),
                    EventKind::BufUnload(ev) => ev.cleanup(key, events),
                    EventKind::BufWritePost(ev) => ev.cleanup(key, events),
                    EventKind::CursorMoved(ev) => ev.cleanup(key, events),
                    EventKind::ModeChanged(ev) => ev.cleanup(key, events),
                    EventKind::OnBytes(ev) => ev.cleanup(key, events),
                    EventKind::UneditableEolSet(ev) => ev.cleanup(key, events),
                }
            }
        })
    }
}

pub(crate) trait CallbacksContainer<Ev: Event> {
    type Key;

    fn get_mut(&mut self, key: Self::Key) -> Option<&mut Callbacks<Ev>>;
    fn insert(&mut self, key: Self::Key, callbacks: Callbacks<Ev>);
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
