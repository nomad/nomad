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
pub(crate) struct BufEnter;

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
