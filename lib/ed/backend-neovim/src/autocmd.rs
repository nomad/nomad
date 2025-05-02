use core::any;

use ed_core::Shared;
use ed_core::backend::AgentId;
use nohash::IntMap;
use slotmap::{DefaultKey, SlotMap};

use crate::{NeovimBuffer, oxi};

/// TODO: docs.
pub struct EventHandle {
    callbacks: Callbacks,
    kind: EventHandleKind,
    event_key: DefaultKey,
}

pub(crate) trait Autocmd {
    type Args<'a>;

    fn get_or_insert_callbacks<'cbs>(
        &self,
        callbacks: &'cbs mut CallbacksInner,
    ) -> &'cbs mut AutocmdCallbacks<Self>;

    fn kind(&self) -> EventHandleKind;

    fn register(&self, callbacks: Callbacks) -> u32;
}

#[derive(Default, Clone)]
pub(crate) struct Callbacks {
    inner: Shared<CallbacksInner>,
}

#[derive(Default)]
pub(crate) struct AgentIds {
    pub(crate) created_buffer: IntMap<NeovimBuffer, AgentId>,
}

pub(crate) struct OnBufferCreated;

#[derive(Default)]
struct CallbacksInner {
    agent_ids: AgentIds,
    on_buffer_created: AutocmdCallbacks<OnBufferCreated>,
}

#[derive(Default)]
enum AutocmdCallbacks<T: Autocmd + ?Sized> {
    #[default]
    Unregistered,
    Registered {
        autocmd_id: u32,
        callbacks: SlotMap<DefaultKey, Box<dyn FnMut(T::Args<'_>) + 'static>>,
    },
}

enum EventHandleKind {
    OnBufferCreated,
}

impl Callbacks {
    pub(crate) fn insert_callback_for<T: Autocmd>(
        &mut self,
        autocmd: T,
        fun: impl FnMut(T::Args<'_>) + 'static,
    ) -> EventHandle {
        EventHandle {
            callbacks: self.clone(),
            kind: autocmd.kind(),
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
            let _ = oxi::api::del_autocmd(*autocmd_id);
        }
    }
}

impl Autocmd for OnBufferCreated {
    type Args<'a> = &'a NeovimBuffer;

    fn get_or_insert_callbacks<'cbs>(
        &self,
        callbacks: &'cbs mut CallbacksInner,
    ) -> &'cbs mut AutocmdCallbacks<Self> {
        &mut callbacks.on_buffer_created
    }

    fn kind(&self) -> EventHandleKind {
        EventHandleKind::OnBufferCreated
    }

    fn register(&self, callbacks: Callbacks) -> u32 {
        let opts = oxi::api::opts::CreateAutocmdOpts::builder()
            .callback(move |args: oxi::api::types::AutocmdCallbackArgs| {
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

        oxi::api::create_autocmd(["BufReadPost"], &opts)
            .expect("couldn't create autocmd")
    }
}

impl Drop for EventHandle {
    #[inline]
    fn drop(&mut self) {
        self.callbacks.inner.with_mut(|callbacks| match self.kind {
            EventHandleKind::OnBufferCreated => {
                callbacks.on_buffer_created.remove(self.event_key);
            },
        })
    }
}
