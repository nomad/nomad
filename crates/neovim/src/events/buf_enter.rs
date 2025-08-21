use editor::{AccessMut, AgentId, Editor};

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
                    .on_buffer_focused
                    .as_ref()
                    .map(|cbs| cbs.cloned())
                else {
                    return true;
                };

                let focused_by = nvim
                    .events2
                    .agent_ids
                    .focused_buffer
                    .remove(&buffer_id)
                    .unwrap_or(AgentId::UNKNOWN);

                let Some(buffer) = nvim.buffer(buffer_id) else {
                    tracing::error!(
                        buffer_id = ?buffer_id,
                        "BufEnter triggered for an invalid buffer",
                    );
                    return false;
                };

                for callback in callbacks {
                    callback((&buffer, focused_by));
                }

                false
            })
        };

        api::create_autocmd(
            ["BufEnter"],
            &api::opts::CreateAutocmdOpts::builder()
                .group(events.augroup_id)
                .callback(oxi::Function::from_fn_mut(callback))
                .build(),
        )
        .expect("couldn't create autocmd on BufEnter")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}
