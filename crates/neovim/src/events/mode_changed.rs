use editor::{AccessMut, AgentId, Editor};

use crate::Neovim;
use crate::buffer::{BufferId, NeovimBuffer};
use crate::events::{AutocmdId, Callbacks, Event, EventKind, Events};
use crate::mode::ModeStr;
use crate::oxi::api;
use crate::utils::CallbackExt;

#[derive(Clone, Copy)]
pub(crate) struct ModeChanged;

impl Event for ModeChanged {
    type Args<'a> = (NeovimBuffer<'a>, ModeStr<'a>, ModeStr<'a>, AgentId);
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
                    .on_mode_changed
                    .as_ref()
                    .map(|cbs| cbs.cloned())
                else {
                    return true;
                };

                let Some(mut buffer) = nvim.buffer(buffer_id) else {
                    tracing::error!(
                        buffer_name = ?args.buffer.get_name().ok(),
                        "ModeChanged triggered for an invalid buffer",
                    );
                    return true;
                };

                let (old_mode, new_mode) =
                    args.r#match.split_once(':').expect(
                        "expected a string with format \
                         \"{{old_mode}}:{{new_mode}}\"",
                    );

                let old_mode = ModeStr::new(old_mode);
                let new_mode = ModeStr::new(new_mode);

                for callback in callbacks {
                    callback((
                        buffer.reborrow(),
                        old_mode,
                        new_mode,
                        AgentId::UNKNOWN,
                    ));
                }

                false
            })
        })
        .catch_unwind()
        .map(|maybe_detach| maybe_detach.unwrap_or(true))
        .into_function();

        api::create_autocmd(
            ["ModeChanged"],
            &api::opts::CreateAutocmdOpts::builder()
                .group(events.augroup_id)
                .callback(callback)
                .build(),
        )
        .expect("couldn't create autocmd on ModeChanged")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}
