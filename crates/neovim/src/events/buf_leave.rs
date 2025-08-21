use editor::AgentId;
use nohash::IntMap as NoHashMap;

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
pub(crate) struct BufLeave(pub(crate) BufferId);

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
