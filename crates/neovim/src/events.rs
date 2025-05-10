use core::{any, mem};

use ed::Shared;
use ed::backend::{AgentId, Buffer, Edit};
use nohash::IntMap as NoHashMap;
use slotmap::{DefaultKey, SlotMap};
use smallvec::smallvec_inline;

use crate::buffer::{BufferId, NeovimBuffer};
use crate::oxi::api::{self, opts, types};

/// TODO: docs.
pub struct EventHandle {
    event_key: DefaultKey,
    event_kind: EventKind,
    events: Shared<Events>,
}

pub(crate) trait Event: Clone + Into<EventKind> {
    /// The type of arguments given to the callbacks registered for this
    /// event.
    type Args<'a>;

    /// The output of [`register()`](Event::register)ing the event.
    type RegisterOutput;

    /// TODO: docs.
    fn get_or_insert_callbacks<'ev>(
        &self,
        events: &'ev mut Events,
    ) -> &'ev mut EventCallbacks<Self>;

    /// TODO: docs.
    fn register(&self, events: Shared<Events>) -> Self::RegisterOutput;

    /// TODO: docs.
    fn unregister(out: Self::RegisterOutput);

    /// TODO: docs.
    fn cleanup(&self, event_key: DefaultKey, events: &mut Events);
}

pub(crate) struct Events {
    pub(crate) agent_ids: AgentIds,
    augroup_id: u32,
    on_buffer_created: EventCallbacks<BufReadPost>,
    on_buffer_edited: NoHashMap<BufferId, EventCallbacks<OnBytes>>,
    on_buffer_removed: NoHashMap<BufferId, EventCallbacks<BufUnload>>,
    on_buffer_saved: NoHashMap<BufferId, EventCallbacks<BufWritePost>>,
}

#[derive(Default)]
pub(crate) struct AgentIds {
    pub(crate) created_buffer: NoHashMap<BufferId, AgentId>,
    pub(crate) edited_buffer: NoHashMap<BufferId, AgentId>,
    pub(crate) removed_buffer: NoHashMap<BufferId, AgentId>,
    pub(crate) saved_buffer: NoHashMap<BufferId, AgentId>,
}

#[derive(Default)]
#[doc(hidden)]
pub(crate) enum EventCallbacks<T: Event> {
    Registered {
        #[allow(clippy::type_complexity)]
        callbacks: SlotMap<DefaultKey, Box<dyn FnMut(T::Args<'_>) + 'static>>,
        output: T::RegisterOutput,
    },
    #[default]
    Unregistered,
}

#[derive(Clone, Copy)]
pub(crate) struct BufReadPost;

#[derive(Clone, Copy)]
pub(crate) struct BufUnload(pub(crate) BufferId);

#[derive(Clone, Copy)]
pub(crate) struct BufWritePost(pub(crate) BufferId);

#[derive(Clone, Copy)]
pub(crate) struct OnBytes(pub(crate) BufferId);

#[derive(cauchy::From)]
pub(crate) enum EventKind {
    BufReadPost(#[from] BufReadPost),
    BufUnload(#[from] BufUnload),
    BufWritePost(#[from] BufWritePost),
    OnBytes(#[from] OnBytes),
}

impl Events {
    pub(crate) fn new(augroup_name: &str) -> Self {
        Self {
            augroup_id: api::create_augroup(
                augroup_name,
                &opts::CreateAugroupOpts::builder().clear(true).build(),
            )
            .expect("couldn't create augroup"),
            agent_ids: Default::default(),
            on_buffer_created: Default::default(),
            on_buffer_edited: Default::default(),
            on_buffer_removed: Default::default(),
            on_buffer_saved: Default::default(),
        }
    }

    pub(crate) fn insert<T: Event>(
        events: Shared<Self>,
        event: T,
        fun: impl FnMut(T::Args<'_>) + 'static,
    ) -> EventHandle {
        let event_kind = event.clone().into();

        let event_key = events.with_mut(|this| {
            match event.get_or_insert_callbacks(this) {
                callbacks @ EventCallbacks::Unregistered => {
                    let mut slotmap = SlotMap::new();
                    let event_key = slotmap.insert(Box::new(fun) as Box<_>);
                    *callbacks = EventCallbacks::Registered {
                        callbacks: slotmap,
                        output: event.register(events.clone()),
                    };
                    event_key
                },
                EventCallbacks::Registered { callbacks, .. } => {
                    callbacks.insert(Box::new(fun))
                },
            }
        });

        EventHandle { event_key, event_kind, events }
    }
}

impl<T: Event> EventCallbacks<T> {
    #[inline]
    fn is_empty(&self) -> bool {
        match self {
            Self::Unregistered => true,
            Self::Registered { callbacks, .. } => callbacks.is_empty(),
        }
    }

    #[track_caller]
    #[inline]
    fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut impl FnMut(T::Args<'_>)> + '_ {
        match self {
            Self::Unregistered => panic!(
                "the autocommand for {} has not been registered",
                any::type_name::<T>()
            ),
            Self::Registered { callbacks, .. } => callbacks.values_mut(),
        }
    }

    #[inline]
    fn remove(&mut self, callback_key: DefaultKey) {
        if let Self::Registered { callbacks, .. } = self {
            callbacks.remove(callback_key);

            // If all the EventHandles have been dropped that means no one
            // cares about the event anymore, and we can unregister it.
            if callbacks.is_empty() {
                match mem::replace(self, Self::Unregistered) {
                    Self::Registered { output, .. } => T::unregister(output),
                    Self::Unregistered => unreachable!("just checked"),
                }
            }
        }
    }
}

impl Event for BufReadPost {
    type Args<'a> = (&'a NeovimBuffer<'a>, AgentId);
    type RegisterOutput = u32;

    #[inline]
    fn get_or_insert_callbacks<'ev>(
        &self,
        events: &'ev mut Events,
    ) -> &'ev mut EventCallbacks<Self> {
        &mut events.on_buffer_created
    }

    #[inline]
    fn register(&self, events: Shared<Events>) -> u32 {
        let opts = opts::CreateAutocmdOpts::builder()
            .group(events.with(|events| events.augroup_id))
            .callback(move |args: types::AutocmdCallbackArgs| {
                events.with_mut(|inner| {
                    let buffer =
                        NeovimBuffer::new(BufferId::new(args.buffer), &events);

                    let created_by = inner
                        .agent_ids
                        .created_buffer
                        .remove(&buffer.id())
                        .unwrap_or(AgentId::UNKNOWN);

                    for callback in inner.on_buffer_created.iter_mut() {
                        callback((&buffer, created_by));
                    }

                    false
                })
            })
            .build();

        api::create_autocmd(["BufReadPost"], &opts)
            .expect("couldn't create autocmd")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }

    #[inline]
    fn cleanup(&self, event_key: DefaultKey, events: &mut Events) {
        events.on_buffer_created.remove(event_key);
    }
}

impl Event for BufUnload {
    type Args<'a> = (&'a NeovimBuffer<'a>, AgentId);
    type RegisterOutput = u32;

    #[inline]
    fn get_or_insert_callbacks<'ev>(
        &self,
        events: &'ev mut Events,
    ) -> &'ev mut EventCallbacks<Self> {
        events.on_buffer_removed.entry(self.0).or_default()
    }

    #[inline]
    fn register(&self, events: Shared<Events>) -> u32 {
        let opts = opts::CreateAutocmdOpts::builder()
            .group(events.with(|events| events.augroup_id))
            .buffer(self.0.into())
            .callback(move |args: types::AutocmdCallbackArgs| {
                events.with_mut(|inner| {
                    let buffer =
                        NeovimBuffer::new(BufferId::new(args.buffer), &events);

                    let Some(callbacks) =
                        inner.on_buffer_saved.get_mut(&buffer.id())
                    else {
                        return true;
                    };

                    let removed_by = inner
                        .agent_ids
                        .removed_buffer
                        .remove(&buffer.id())
                        .unwrap_or(AgentId::UNKNOWN);

                    for callback in callbacks.iter_mut() {
                        callback((&buffer, removed_by));
                    }

                    false
                })
            })
            .build();

        api::create_autocmd(["BufWritePost"], &opts)
            .expect("couldn't create autocmd")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }

    #[inline]
    fn cleanup(&self, event_key: DefaultKey, events: &mut Events) {
        if let Some(callbacks) = events.on_buffer_removed.get_mut(&self.0) {
            callbacks.remove(event_key);
            if callbacks.is_empty() {
                events.on_buffer_removed.remove(&self.0);
            }
        }
    }
}

impl Event for BufWritePost {
    type Args<'a> = (&'a NeovimBuffer<'a>, AgentId);
    type RegisterOutput = u32;

    #[inline]
    fn get_or_insert_callbacks<'ev>(
        &self,
        events: &'ev mut Events,
    ) -> &'ev mut EventCallbacks<Self> {
        events.on_buffer_saved.entry(self.0).or_default()
    }

    #[inline]
    fn register(&self, events: Shared<Events>) -> u32 {
        let opts = opts::CreateAutocmdOpts::builder()
            .group(events.with(|events| events.augroup_id))
            .buffer(self.0.into())
            .callback(move |args: types::AutocmdCallbackArgs| {
                events.with_mut(|inner| {
                    let buffer =
                        NeovimBuffer::new(BufferId::new(args.buffer), &events);

                    let Some(callbacks) =
                        inner.on_buffer_saved.get_mut(&buffer.id())
                    else {
                        return true;
                    };

                    let saved_by = inner
                        .agent_ids
                        .saved_buffer
                        .remove(&buffer.id())
                        .unwrap_or(AgentId::UNKNOWN);

                    for callback in callbacks.iter_mut() {
                        callback((&buffer, saved_by));
                    }

                    false
                })
            })
            .build();

        api::create_autocmd(["BufWritePost"], &opts)
            .expect("couldn't create autocmd")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }

    #[inline]
    fn cleanup(&self, event_key: DefaultKey, events: &mut Events) {
        if let Some(callbacks) = events.on_buffer_saved.get_mut(&self.0) {
            callbacks.remove(event_key);
            if callbacks.is_empty() {
                events.on_buffer_saved.remove(&self.0);
            }
        }
    }
}

impl Event for OnBytes {
    type Args<'a> = (&'a NeovimBuffer<'a>, &'a Edit);
    type RegisterOutput = ();

    #[inline]
    fn get_or_insert_callbacks<'ev>(
        &self,
        events: &'ev mut Events,
    ) -> &'ev mut EventCallbacks<Self> {
        events.on_buffer_edited.entry(self.0).or_default()
    }

    #[inline]
    fn register(&self, events: Shared<Events>) {
        let buffer_id = self.0;

        let opts = opts::BufAttachOpts::builder()
            .on_bytes(move |args: opts::OnBytesArgs| {
                events.with_mut(|inner| {
                    let buffer = NeovimBuffer::new(buffer_id, &events);

                    let Some(callbacks) =
                        inner.on_buffer_edited.get_mut(&buffer.id())
                    else {
                        return true;
                    };

                    let edited_by = inner
                        .agent_ids
                        .edited_buffer
                        .remove(&buffer.id())
                        .unwrap_or(AgentId::UNKNOWN);

                    let edit = Edit {
                        made_by: edited_by,
                        replacements: smallvec_inline![
                            buffer.replacement_of_on_bytes(args)
                        ],
                    };

                    for callback in callbacks.iter_mut() {
                        callback((&buffer, &edit));
                    }

                    false
                })
            })
            .build();

        api::Buffer::from(buffer_id)
            .attach(false, &opts)
            .expect("couldn't attach to buffer");
    }

    #[inline]
    fn unregister((): Self::RegisterOutput) {}

    #[inline]
    fn cleanup(&self, event_key: DefaultKey, events: &mut Events) {
        if let Some(callbacks) = events.on_buffer_edited.get_mut(&self.0) {
            callbacks.remove(event_key);
            if callbacks.is_empty() {
                events.on_buffer_edited.remove(&self.0);
            }
        }
    }
}

impl Drop for EventHandle {
    #[inline]
    fn drop(&mut self) {
        let key = self.event_key;
        self.events.with_mut(|events| match self.event_kind {
            EventKind::BufReadPost(event) => event.cleanup(key, events),
            EventKind::BufUnload(event) => event.cleanup(key, events),
            EventKind::BufWritePost(event) => event.cleanup(key, events),
            EventKind::OnBytes(event) => event.cleanup(key, events),
        })
    }
}
