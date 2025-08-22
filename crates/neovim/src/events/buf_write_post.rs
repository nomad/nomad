use editor::{AccessMut, AgentId, Editor};
use nohash::IntMap as NoHashMap;

use crate::Neovim;
use crate::buffer::{BufferId, NeovimBuffer};
use crate::events::{AutocmdId, Callbacks, Event, EventKind, Events};
use crate::oxi::api;
use crate::utils::CallbackExt;

#[derive(Clone, Copy)]
pub(crate) struct BufWritePost(pub(crate) BufferId);

impl Event for BufWritePost {
    type Args<'a> = (NeovimBuffer<'a>, AgentId);
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
    fn register(
        &self,
        events: &Events,
        mut nvim: impl AccessMut<Neovim> + 'static,
    ) -> AutocmdId {
        let callback = (move |args: api::types::AutocmdCallbackArgs| {
            nvim.with_mut(|nvim| {
                let buffer_id = BufferId::new(args.buffer.clone());

                let Some(callbacks) = nvim
                    .events
                    .on_buffer_saved
                    .get(&buffer_id)
                    .map(|cbs| cbs.cloned())
                else {
                    return true;
                };

                let saved_by = nvim
                    .events
                    .agent_ids
                    .saved_buffer
                    .remove(&buffer_id)
                    .unwrap_or(AgentId::UNKNOWN);

                let Some(mut buffer) = nvim.buffer(buffer_id) else {
                    tracing::error!(
                        buffer_name = ?args.buffer.get_name().ok(),
                        "BufWritePost triggered for an invalid buffer",
                    );
                    return true;
                };

                for callback in callbacks {
                    callback((buffer.reborrow(), saved_by));
                }

                false
            })
        })
        .catch_unwind()
        .map(|maybe_detach| maybe_detach.unwrap_or(true))
        .into_function();

        api::create_autocmd(
            ["BufWritePost"],
            &api::opts::CreateAutocmdOpts::builder()
                .group(events.augroup_id)
                .buffer(self.0.into())
                .callback(callback)
                .build(),
        )
        .expect("couldn't create autocmd on BufWritePost")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}
