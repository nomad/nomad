use editor::{AccessMut, AgentId, Editor};
use nohash::IntMap as NoHashMap;

use crate::Neovim;
use crate::buffer::{BufferExt, BufferId};
use crate::cursor::NeovimCursor;
use crate::events::{
    AutocmdId,
    Callbacks,
    Event,
    EventKind,
    Events,
    ModeChanged,
};
use crate::oxi::api;
use crate::utils::CallbackExt;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CursorMoved(pub(crate) BufferId);

impl Event for CursorMoved {
    type Args<'a> = (NeovimCursor<'a>, AgentId);
    type Container<'ev> = &'ev mut NoHashMap<BufferId, Callbacks<Self>>;
    type RegisterOutput = AutocmdId;

    #[inline]
    fn container<'ev>(&self, events: &'ev mut Events) -> Self::Container<'ev> {
        &mut events.on_cursor_moved
    }

    #[inline]
    fn key(&self) -> BufferId {
        self.0
    }

    #[inline]
    fn kind(&self) -> EventKind {
        EventKind::CursorMoved(*self)
    }

    #[inline]
    fn register(
        &self,
        events: &Events,
        mut nvim: impl AccessMut<Neovim> + 'static,
    ) -> AutocmdId {
        let callback = (move |args: api::types::AutocmdCallbackArgs| {
            if args.event == "ModeChanged" {
                let (old_mode, new_mode) = ModeChanged::parse_args(&args);
                // We only care about ModeChanged events if transitioning from
                // or to insert mode.
                if !old_mode.is_insert() && !new_mode.is_insert() {
                    return false;
                }
            }

            nvim.with_mut(|nvim| {
                let buffer_id = BufferId::from(args.buffer.clone());

                let Some(callbacks) = nvim
                    .events
                    .on_cursor_moved
                    .get(&buffer_id)
                    .map(|cbs| cbs.cloned())
                else {
                    return true;
                };

                let moved_by = nvim
                    .events
                    .agent_ids
                    .moved_cursor
                    .remove(&buffer_id)
                    .unwrap_or(AgentId::UNKNOWN);

                let Some(buffer) = nvim.buffer(buffer_id) else {
                    tracing::error!(
                        buffer_name = %args.buffer.name(),
                        "{:?} triggered for an invalid buffer", args.event,
                    );
                    return true;
                };

                let mut cursor = NeovimCursor::from(buffer);

                for callback in callbacks {
                    callback((cursor.reborrow(), moved_by));
                }

                false
            })
        })
        .catch_unwind()
        .map(|maybe_detach| maybe_detach.unwrap_or(true))
        .into_function();

        // Neovim has 3 separate cursor-move-related autocommand events --
        // CursorMoved, CursorMovedI and CursorMovedC -- which are triggered
        // when the cursor is moved in Normal/Visual mode, Insert mode and in
        // the command line, respectively.
        //
        // Since our editor API has no concept of modes, we register the
        // callback on both CursorMoved and CursorMovedI.
        //
        // We register on WinEnter because navigating between different window
        // splits all displaying the same buffer is the same as moving the
        // cursor.
        //
        // We register on ModeChanged because the cursor moves one character to
        // the left when going from insert mode to normal mode and one
        // character to the right when going from normal mode to insert mode
        // with "a".
        api::create_autocmd(
            ["CursorMoved", "CursorMovedI", "WinEnter", "ModeChanged"],
            &api::opts::CreateAutocmdOpts::builder()
                .group(events.augroup_id)
                .buffer(self.0.into())
                .callback(callback)
                .build(),
        )
        .expect("couldn't create autocmd")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}
