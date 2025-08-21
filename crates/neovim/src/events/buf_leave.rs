use editor::{AccessMut, AgentId, Editor};
use nohash::IntMap as NoHashMap;

use crate::Neovim;
use crate::buffer::{BufferId, NeovimBuffer};
use crate::events::{
    AutocmdId,
    Callbacks,
    Event,
    EventKind,
    Events,
    EventsBorrow,
};
use crate::oxi::{self, api};

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
    fn register(&self, _: EventsBorrow) -> AutocmdId {
        todo!();
    }

    #[inline]
    fn register2(
        &self,
        events: &mut Events,
        mut nvim: impl AccessMut<Neovim> + 'static,
    ) -> AutocmdId {
        let callback = move |args: api::types::AutocmdCallbackArgs| {
            nvim.with_mut(|nvim| {
                let buffer_id = BufferId::new(args.buffer);

                let Some(callbacks) = nvim
                    .events2
                    .on_buffer_unfocused
                    .get(&buffer_id)
                    .map(|cbs| cbs.cloned())
                else {
                    return true;
                };

                let removed_by = AgentId::UNKNOWN;

                let Some(buffer) = nvim.buffer(buffer_id) else {
                    tracing::error!(
                        buffer_id = ?buffer_id,
                        "BufLeave triggered for an invalid buffer",
                    );
                    return false;
                };

                for callback in callbacks {
                    callback((&buffer, removed_by));
                }

                false
            })
        };

        api::create_autocmd(
            ["BufLeave"],
            &api::opts::CreateAutocmdOpts::builder()
                .group(events.augroup_id)
                .buffer(self.0.into())
                .callback(oxi::Function::from_fn_mut(callback))
                .build(),
        )
        .expect("couldn't create autocmd on BufLeave")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}
