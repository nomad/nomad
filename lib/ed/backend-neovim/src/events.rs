use core::{any, mem};

use ed_core::Shared;
use ed_core::backend::{AgentId, Buffer, Edit};
use nohash::IntMap as NoHashMap;
use slotmap::{DefaultKey, SlotMap};
use smallvec::smallvec_inline;

use crate::buffer::{BufferId, NeovimBuffer};
use crate::oxi::api::{self, opts, types};

/// TODO: docs.
pub struct EventHandle {
    callbacks: Callbacks,
    kind: EventKind,
    event_key: DefaultKey,
}

pub(crate) trait Event: Clone + Into<EventKind> {
    type Args<'a>;

    /// The output of [`register()`](Event::register)ing the event.
    type RegisterOutput;

    #[doc(hidden)]
    fn get_or_insert_callbacks<'cbs>(
        &self,
        callbacks: &'cbs mut CallbacksInner,
    ) -> &'cbs mut AutocmdCallbacks<Self>;

    #[doc(hidden)]
    fn register(&self, callbacks: Callbacks) -> Self::RegisterOutput;

    #[doc(hidden)]
    fn unregister(out: Self::RegisterOutput);

    #[doc(hidden)]
    fn cleanup(&self, event_key: DefaultKey, callbacks: &mut CallbacksInner);
}

#[derive(Default, Clone)]
pub(crate) struct Callbacks {
    inner: Shared<CallbacksInner>,
}

#[derive(Default)]
pub(crate) struct AgentIds {
    pub(crate) created_buffer: NoHashMap<BufferId, AgentId>,
    pub(crate) edited_buffer: NoHashMap<BufferId, AgentId>,
    pub(crate) removed_buffer: NoHashMap<BufferId, AgentId>,
    pub(crate) saved_buffer: NoHashMap<BufferId, AgentId>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufReadPost;

#[derive(Clone, Copy)]
pub(crate) struct BufUnload(pub(crate) BufferId);

#[derive(Clone, Copy)]
pub(crate) struct BufWritePost(pub(crate) BufferId);

#[derive(Clone, Copy)]
pub(crate) struct OnBytes(pub(crate) BufferId);

#[derive(Default)]
#[doc(hidden)]
pub(crate) struct CallbacksInner {
    agent_ids: AgentIds,
    on_buffer_created: AutocmdCallbacks<BufReadPost>,
    on_buffer_edited: NoHashMap<BufferId, AutocmdCallbacks<OnBytes>>,
    on_buffer_removed: NoHashMap<BufferId, AutocmdCallbacks<BufUnload>>,
    on_buffer_saved: NoHashMap<BufferId, AutocmdCallbacks<BufWritePost>>,
}

#[derive(Default)]
#[doc(hidden)]
pub(crate) enum AutocmdCallbacks<T: Event> {
    Registered {
        #[allow(clippy::type_complexity)]
        callbacks: SlotMap<DefaultKey, Box<dyn FnMut(T::Args<'_>) + 'static>>,
        output: T::RegisterOutput,
    },
    #[default]
    Unregistered,
}

#[derive(cauchy::From)]
pub(crate) enum EventKind {
    BufReadPost(#[from] BufReadPost),
    BufUnload(#[from] BufUnload),
    BufWritePost(#[from] BufWritePost),
    OnBytes(#[from] OnBytes),
}

impl Callbacks {
    pub(crate) fn insert_callback_for<T: Event>(
        &self,
        event: T,
        fun: impl FnMut(T::Args<'_>) + 'static,
    ) -> EventHandle {
        EventHandle {
            callbacks: self.clone(),
            kind: event.clone().into(),
            event_key: self.inner.with_mut(|inner| {
                inner.insert_callback_for(event, fun, self.clone())
            }),
        }
    }
}

impl CallbacksInner {
    pub(crate) fn insert_callback_for<T: Event>(
        &mut self,
        event: T,
        fun: impl FnMut(T::Args<'_>) + 'static,
        callbacks: Callbacks,
    ) -> DefaultKey {
        let autocmd_callbacks = event.get_or_insert_callbacks(self);

        match autocmd_callbacks {
            AutocmdCallbacks::Unregistered => {
                let output = event.register(callbacks);
                let mut callbacks = SlotMap::new();
                let key = callbacks.insert(Box::new(fun) as Box<_>);
                *autocmd_callbacks =
                    AutocmdCallbacks::Registered { callbacks, output };
                key
            },
            AutocmdCallbacks::Registered { callbacks, .. } => {
                callbacks.insert(Box::new(fun))
            },
        }
    }
}

impl<T: Event> AutocmdCallbacks<T> {
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
    type Args<'a> = &'a NeovimBuffer<'a>;
    type RegisterOutput = u32;

    #[inline]
    fn get_or_insert_callbacks<'cbs>(
        &self,
        callbacks: &'cbs mut CallbacksInner,
    ) -> &'cbs mut AutocmdCallbacks<Self> {
        &mut callbacks.on_buffer_created
    }

    #[inline]
    fn register(&self, callbacks: Callbacks) -> u32 {
        let opts = opts::CreateAutocmdOpts::builder()
            .callback(move |args: types::AutocmdCallbackArgs| {
                callbacks.inner.with_mut(|inner| {
                    let buffer = NeovimBuffer::new(
                        BufferId::new(args.buffer),
                        &callbacks,
                    );

                    let _created_by = inner
                        .agent_ids
                        .created_buffer
                        .remove(&buffer.id())
                        .unwrap_or(AgentId::UNKNOWN);

                    for callback in inner.on_buffer_created.iter_mut() {
                        callback(&buffer);
                    }

                    false
                })
            })
            .build();

        api::create_autocmd(["BufReadPost"], &opts)
            .expect("couldn't create autocmd")
    }

    #[inline]
    fn unregister(out: Self::RegisterOutput) {
        let _ = api::del_autocmd(out);
    }

    #[inline]
    fn cleanup(&self, event_key: DefaultKey, callbacks: &mut CallbacksInner) {
        callbacks.on_buffer_created.remove(event_key);
    }
}

impl Event for BufUnload {
    type Args<'a> = (&'a NeovimBuffer<'a>, AgentId);
    type RegisterOutput = u32;

    #[inline]
    fn get_or_insert_callbacks<'cbs>(
        &self,
        callbacks: &'cbs mut CallbacksInner,
    ) -> &'cbs mut AutocmdCallbacks<Self> {
        callbacks.on_buffer_removed.entry(self.0).or_default()
    }

    #[inline]
    fn register(&self, callbacks: Callbacks) -> u32 {
        let opts = opts::CreateAutocmdOpts::builder()
            .buffer(self.0.into())
            .callback(move |args: types::AutocmdCallbackArgs| {
                callbacks.inner.with_mut(|inner| {
                    let buffer = NeovimBuffer::new(
                        BufferId::new(args.buffer),
                        &callbacks,
                    );

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
    fn unregister(out: Self::RegisterOutput) {
        let _ = api::del_autocmd(out);
    }

    #[inline]
    fn cleanup(&self, event_key: DefaultKey, inner: &mut CallbacksInner) {
        if let Some(callbacks) = inner.on_buffer_removed.get_mut(&self.0) {
            callbacks.remove(event_key);
            if callbacks.is_empty() {
                inner.on_buffer_removed.remove(&self.0);
            }
        }
    }
}

impl Event for BufWritePost {
    type Args<'a> = (&'a NeovimBuffer<'a>, AgentId);
    type RegisterOutput = u32;

    #[inline]
    fn get_or_insert_callbacks<'cbs>(
        &self,
        callbacks: &'cbs mut CallbacksInner,
    ) -> &'cbs mut AutocmdCallbacks<Self> {
        callbacks.on_buffer_saved.entry(self.0).or_default()
    }

    #[inline]
    fn register(&self, callbacks: Callbacks) -> u32 {
        let opts = opts::CreateAutocmdOpts::builder()
            .buffer(self.0.into())
            .callback(move |args: types::AutocmdCallbackArgs| {
                callbacks.inner.with_mut(|inner| {
                    let buffer = NeovimBuffer::new(
                        BufferId::new(args.buffer),
                        &callbacks,
                    );

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
    fn unregister(out: Self::RegisterOutput) {
        let _ = api::del_autocmd(out);
    }

    #[inline]
    fn cleanup(&self, event_key: DefaultKey, inner: &mut CallbacksInner) {
        if let Some(callbacks) = inner.on_buffer_saved.get_mut(&self.0) {
            callbacks.remove(event_key);
            if callbacks.is_empty() {
                inner.on_buffer_saved.remove(&self.0);
            }
        }
    }
}

impl Event for OnBytes {
    type Args<'a> = (&'a NeovimBuffer<'a>, &'a Edit);
    type RegisterOutput = ();

    #[inline]
    fn get_or_insert_callbacks<'cbs>(
        &self,
        callbacks: &'cbs mut CallbacksInner,
    ) -> &'cbs mut AutocmdCallbacks<Self> {
        callbacks.on_buffer_edited.entry(self.0).or_default()
    }

    #[inline]
    fn register(&self, callbacks: Callbacks) {
        let buffer_id = self.0;

        let opts = opts::BufAttachOpts::builder()
            .on_bytes(move |args: opts::OnBytesArgs| {
                callbacks.inner.with_mut(|inner| {
                    let buffer = NeovimBuffer::new(buffer_id, &callbacks);

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

        api::Buffer::from(self.0)
            .attach(false, &opts)
            .expect("couldn't attach to buffer");
    }

    #[inline]
    fn unregister((): Self::RegisterOutput) {}

    #[inline]
    fn cleanup(&self, event_key: DefaultKey, inner: &mut CallbacksInner) {
        if let Some(callbacks) = inner.on_buffer_edited.get_mut(&self.0) {
            callbacks.remove(event_key);
            if callbacks.is_empty() {
                inner.on_buffer_edited.remove(&self.0);
            }
        }
    }
}

impl Drop for EventHandle {
    #[inline]
    fn drop(&mut self) {
        let key = self.event_key;
        self.callbacks.inner.with_mut(|inner| match self.kind {
            EventKind::BufReadPost(event) => event.cleanup(key, inner),
            EventKind::BufUnload(event) => event.cleanup(key, inner),
            EventKind::BufWritePost(event) => event.cleanup(key, inner),
            EventKind::OnBytes(event) => event.cleanup(key, inner),
        })
    }
}
