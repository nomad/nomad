use editor::{AccessMut, AgentId, Edit, Editor};
use nohash::IntMap as NoHashMap;
use smallvec::smallvec_inline;

use crate::Neovim;
use crate::buffer::{BufferId, NeovimBuffer};
use crate::events::{Callbacks, Event, EventKind, Events};
use crate::oxi::{self, api};

#[derive(Clone, Copy)]
pub(crate) struct OnBytes(pub(crate) BufferId);

impl Event for OnBytes {
    type Args<'a> = (NeovimBuffer<'a>, &'a Edit);
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
    fn register(
        &self,
        _: &Events,
        mut nvim: impl AccessMut<Neovim> + 'static,
    ) {
        let buffer_id = self.0;

        let callback = move |args: api::opts::OnBytesArgs| {
            nvim.with_mut(|nvim| {
                let Some(callbacks) = nvim
                    .events
                    .on_buffer_edited
                    .get(&buffer_id)
                    .map(|cbs| cbs.cloned())
                else {
                    return true;
                };

                let edited_by = nvim
                    .events
                    .agent_ids
                    .edited_buffer
                    .remove(&buffer_id)
                    .unwrap_or(AgentId::UNKNOWN) ;

                let Some(mut buffer) = nvim.buffer(buffer_id) else {
                    tracing::error!(
                        buffer_name = ?api::Buffer::from(buffer_id).get_name().ok(),
                        "OnBytes triggered for an invalid buffer",
                    );
                    return true;
                };

                let edit = Edit {
                    made_by: edited_by,
                    replacements: smallvec_inline![
                        buffer.replacement_of_on_bytes(args)
                    ],
                };

                for callback in callbacks {
                    callback((buffer.reborrow(), &edit));
                }

                false
            })
        };

        api::Buffer::from(buffer_id)
            .attach(
                false,
                &api::opts::BufAttachOpts::builder()
                    .on_bytes(oxi::Function::from_fn_mut(callback))
                    .build(),
            )
            .expect("couldn't attach to buffer");
    }

    #[inline]
    fn unregister((): Self::RegisterOutput) {}
}
