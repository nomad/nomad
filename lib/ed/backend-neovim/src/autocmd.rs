use core::any;

use ed_core::Shared;
use ed_core::backend::AgentId;
use nohash::IntMap as NoHashMap;
use slotmap::{DefaultKey, SlotMap};

use crate::NeovimBuffer;
use crate::oxi::api::{self, opts, types};

/// TODO: docs.
pub struct EventHandle {
    callbacks: Callbacks,
    kind: EventKind,
    event_key: DefaultKey,
}

pub(crate) trait Autocmd: Clone + Into<EventKind> {
    type Args<'a>;

    #[doc(hidden)]
    fn get_or_insert_callbacks<'cbs>(
        &self,
        callbacks: &'cbs mut CallbacksInner,
    ) -> &'cbs mut AutocmdCallbacks<Self>;

    #[doc(hidden)]
    fn register(&self, callbacks: Callbacks) -> u32;

    #[doc(hidden)]
    fn cleanup(&self, event_key: DefaultKey, callbacks: &mut CallbacksInner);
}

#[derive(Default, Clone)]
pub(crate) struct Callbacks {
    inner: Shared<CallbacksInner>,
}

#[derive(Default)]
pub(crate) struct AgentIds {
    pub(crate) created_buffer: NoHashMap<NeovimBuffer, AgentId>,
    pub(crate) edited_buffer: NoHashMap<NeovimBuffer, AgentId>,
    pub(crate) removed_buffer: NoHashMap<NeovimBuffer, AgentId>,
    pub(crate) saved_buffer: NoHashMap<NeovimBuffer, AgentId>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufReadPost;

#[derive(Clone, Copy)]
pub(crate) struct BufWritePost(pub(crate) NeovimBuffer);

#[derive(Default)]
#[doc(hidden)]
pub(crate) struct CallbacksInner {
    agent_ids: AgentIds,
    on_buffer_created: AutocmdCallbacks<BufReadPost>,
    on_buffer_saved: NoHashMap<NeovimBuffer, AutocmdCallbacks<BufWritePost>>,
}

#[derive(Default)]
#[doc(hidden)]
pub(crate) enum AutocmdCallbacks<T: Autocmd> {
    #[default]
    Unregistered,
    Registered {
        autocmd_id: u32,
        #[allow(clippy::type_complexity)]
        callbacks: SlotMap<DefaultKey, Box<dyn FnMut(T::Args<'_>) + 'static>>,
    },
}

#[derive(cauchy::From)]
pub(crate) enum EventKind {
    BufReadPost(#[from] BufReadPost),
    BufWritePost(#[from] BufWritePost),
}

impl Callbacks {
    pub(crate) fn insert_callback_for<T: Autocmd>(
        &mut self,
        autocmd: T,
        fun: impl FnMut(T::Args<'_>) + 'static,
    ) -> EventHandle {
        EventHandle {
            callbacks: self.clone(),
            kind: autocmd.clone().into(),
            event_key: self.inner.with_mut(|inner| {
                inner.insert_callback_for(autocmd, fun, self.clone())
            }),
        }
    }
}

impl CallbacksInner {
    pub(crate) fn insert_callback_for<T: Autocmd>(
        &mut self,
        autocmd: T,
        fun: impl FnMut(T::Args<'_>) + 'static,
        callbacks: Callbacks,
    ) -> DefaultKey {
        let autocmd_callbacks = autocmd.get_or_insert_callbacks(self);

        match autocmd_callbacks {
            AutocmdCallbacks::Unregistered => {
                let autocmd_id = autocmd.register(callbacks);
                let mut callbacks = SlotMap::new();
                let key = callbacks.insert(Box::new(fun) as Box<_>);
                *autocmd_callbacks =
                    AutocmdCallbacks::Registered { autocmd_id, callbacks };
                key
            },
            AutocmdCallbacks::Registered { callbacks, .. } => {
                callbacks.insert(Box::new(fun))
            },
        }
    }
}

impl<T: Autocmd> AutocmdCallbacks<T> {
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
        let Self::Registered { autocmd_id, callbacks } = self else { return };
        callbacks.remove(callback_key);
        if callbacks.is_empty() {
            // All the EventHandles have been dropped, which means no one cares
            // about the event anymore and we can delete the autocommand.
            let _ = api::del_autocmd(*autocmd_id);
        }
    }
}

impl Autocmd for BufReadPost {
    type Args<'a> = &'a NeovimBuffer;

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
                let buffer = NeovimBuffer::new(args.buffer);

                callbacks.inner.with_mut(|callbacks| {
                    let _created_by = callbacks
                        .agent_ids
                        .created_buffer
                        .remove(&buffer)
                        .unwrap_or(AgentId::UNKNOWN);

                    for callback in callbacks.on_buffer_created.iter_mut() {
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
    fn cleanup(&self, event_key: DefaultKey, callbacks: &mut CallbacksInner) {
        callbacks.on_buffer_created.remove(event_key);
    }
}

impl Autocmd for BufWritePost {
    type Args<'a> = (&'a NeovimBuffer, AgentId);

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
                let buffer = NeovimBuffer::new(args.buffer);

                callbacks.inner.with_mut(|callbacks| {
                    let Some(autocmd_callbacks) =
                        callbacks.on_buffer_saved.get_mut(&buffer)
                    else {
                        return true;
                    };

                    let saved_by = callbacks
                        .agent_ids
                        .saved_buffer
                        .remove(&buffer)
                        .unwrap_or(AgentId::UNKNOWN);

                    for callback in autocmd_callbacks.iter_mut() {
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
    fn cleanup(&self, event_key: DefaultKey, callbacks: &mut CallbacksInner) {
        let Some(autocmd_callbacks) =
            callbacks.on_buffer_saved.get_mut(&self.0)
        else {
            return;
        };

        autocmd_callbacks.remove(event_key);

        if autocmd_callbacks.is_empty() {
            callbacks.on_buffer_saved.remove(&self.0);
        }
    }
}

impl Drop for EventHandle {
    #[inline]
    fn drop(&mut self) {
        let key = self.event_key;
        self.callbacks.inner.with_mut(|inner| match self.kind {
            EventKind::BufReadPost(event) => event.cleanup(key, inner),
            EventKind::BufWritePost(event) => event.cleanup(key, inner),
        })
    }
}
