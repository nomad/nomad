use editor::AgentId;

use crate::buffer::{BufferId, NeovimBuffer};
use crate::events::{
    AutocmdId,
    Callbacks,
    Event,
    EventKind,
    Events,
    EventsBorrow,
};
use crate::oxi::api;

#[derive(Clone, Copy)]
pub(crate) struct BufReadPost;

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
