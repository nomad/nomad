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
use crate::mode::ModeStr;
use crate::oxi::api;

#[derive(Clone, Copy)]
pub(crate) struct ModeChanged;

impl Event for ModeChanged {
    type Args<'a> = (&'a NeovimBuffer<'a>, ModeStr<'a>, ModeStr<'a>, AgentId);
    type Container<'ev> = &'ev mut Option<Callbacks<Self>>;
    type RegisterOutput = AutocmdId;

    #[inline]
    fn container<'ev>(&self, events: &'ev mut Events) -> Self::Container<'ev> {
        &mut events.on_mode_changed
    }

    #[inline]
    fn key(&self) {}

    #[inline]
    fn kind(&self) -> EventKind {
        EventKind::ModeChanged(*self)
    }

    #[inline]
    fn register(&self, events: EventsBorrow) -> Self::RegisterOutput {
        let augroup_id = events.augroup_id;

        let bufs_state = events.borrow.buffers_state.clone();
        let events = events.handle;

        let opts = api::opts::CreateAutocmdOpts::builder()
            .group(augroup_id)
            .callback(move |args: api::types::AutocmdCallbackArgs| {
                let buffer_id = BufferId::new(args.buffer);

                let Some(callbacks) = events.with(|ev| {
                    ev.on_mode_changed.as_ref().map(Callbacks::cloned)
                }) else {
                    return true;
                };

                let (old_mode, new_mode) =
                    args.r#match.split_once(':').expect(
                        "expected a string with format \
                         \"{{old_mode}}:{{new_mode}}\"",
                    );

                let buffer = Events::buffer(buffer_id, &events, &bufs_state);
                let old_mode = ModeStr::new(old_mode);
                let new_mode = ModeStr::new(new_mode);

                for callback in callbacks {
                    callback((&buffer, old_mode, new_mode, AgentId::UNKNOWN));
                }

                false
            })
            .build();

        api::create_autocmd(["ModeChanged"], &opts)
            .expect("couldn't create autocmd on ModeChanged")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}
