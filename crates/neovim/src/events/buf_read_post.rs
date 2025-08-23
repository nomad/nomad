use editor::{AccessMut, AgentId, Editor};

use crate::Neovim;
use crate::buffer::{BufferId, NeovimBuffer};
use crate::events::{AutocmdId, Callbacks, Event, EventKind, Events};
use crate::oxi::api;
use crate::utils::CallbackExt;

#[derive(Clone, Copy)]
pub(crate) struct BufReadPost;

impl Event for BufReadPost {
    type Args<'a> = (NeovimBuffer<'a>, AgentId);
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
    fn register(
        &self,
        events: &Events,
        mut nvim: impl AccessMut<Neovim> + 'static,
    ) -> AutocmdId {
        let callback = (move |args: api::types::AutocmdCallbackArgs| {
            nvim.with_mut(|nvim| {
                let buffer_id = BufferId::from(args.buffer.clone());

                let Some(mut buffer) = nvim.buffer(buffer_id) else {
                    return false;
                };

                let events = &mut buffer.nvim.events;

                let Some(callbacks) = &events.on_buffer_created else {
                    return true;
                };

                let created_by = events
                    .agent_ids
                    .created_buffer
                    .remove(&buffer_id)
                    .unwrap_or(AgentId::UNKNOWN);

                for callback in callbacks.cloned() {
                    callback((buffer.reborrow(), created_by));
                }

                false
            })
        })
        .catch_unwind()
        .map(|maybe_detach| maybe_detach.unwrap_or(true))
        .into_function();

        api::create_autocmd(
            ["BufReadPost"],
            &api::opts::CreateAutocmdOpts::builder()
                .group(events.augroup_id)
                .callback(callback)
                .build(),
        )
        .expect("couldn't create autocmd on BufReadPost")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}
