use editor::{AgentId, Edit};
use nohash::IntMap as NoHashMap;
use smallvec::smallvec_inline;

use crate::buffer::{BufferId, NeovimBuffer};
use crate::events::{Callbacks, Event, EventKind, Events, EventsBorrow};
use crate::oxi::api;

#[derive(Clone, Copy)]
pub(crate) struct OnBytes(pub(crate) BufferId);

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
