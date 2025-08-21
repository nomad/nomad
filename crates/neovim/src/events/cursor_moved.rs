use editor::AgentId;
use nohash::IntMap as NoHashMap;

use crate::buffer::BufferId;
use crate::cursor::NeovimCursor;
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
pub(crate) struct CursorMoved(pub(crate) BufferId);

impl Event for CursorMoved {
    type Args<'a> = (&'a NeovimCursor<'a>, AgentId);
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
    fn register(&self, events: EventsBorrow) -> Self::RegisterOutput {
        let augroup_id = events.augroup_id;

        let bufs_state = events.borrow.buffers_state.clone();
        let events = events.handle;

        let opts = api::opts::CreateAutocmdOpts::builder()
            .group(augroup_id)
            .buffer(self.0.into())
            .callback(move |args: api::types::AutocmdCallbackArgs| {
                let buffer_id = BufferId::new(args.buffer);

                let Some((callbacks, moved_by)) = events.with_mut(|ev| {
                    let callbacks = ev.on_cursor_moved.get(&buffer_id)?;

                    let moved_by = ev
                        .agent_ids
                        .moved_cursor
                        .remove(&buffer_id)
                        .unwrap_or(AgentId::UNKNOWN);

                    Some((callbacks.cloned(), moved_by))
                }) else {
                    return true;
                };

                let cursor = NeovimCursor::new(Events::buffer(
                    buffer_id,
                    &events,
                    &bufs_state,
                ));

                for callback in callbacks {
                    callback((&cursor, moved_by));
                }

                false
            })
            .build();

        // Neovim has 3 separate cursor-move-related autocommand events --
        // CursorMoved, CursorMovedI and CursorMovedC -- which are triggered
        // when the cursor is moved in Normal/Visual mode, Insert mode and in
        // the command line, respectively.
        //
        // Since ed has no concept of modes, we register the callback on both
        // CursorMoved and CursorMovedI.

        api::create_autocmd(["CursorMoved", "CursorMovedI"], &opts)
            .expect("couldn't create autocmd on CursorMoved{I}")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}
