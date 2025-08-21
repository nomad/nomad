use editor::{AccessMut, AgentId, Editor};
use nohash::IntMap as NoHashMap;

use crate::Neovim;
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
use crate::oxi::{self, api};

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
                let buffer_id = BufferId::new(args.buffer.clone());

                let Some(callbacks) = nvim
                    .events2
                    .on_cursor_moved
                    .get(&buffer_id)
                    .map(|cbs| cbs.cloned())
                else {
                    return true;
                };

                let moved_by = nvim
                    .events2
                    .agent_ids
                    .moved_cursor
                    .remove(&buffer_id)
                    .unwrap_or(AgentId::UNKNOWN);

                let Some(buffer) = nvim.buffer(buffer_id) else {
                    tracing::error!(
                        buffer_name = ?args.buffer.get_name().ok(),
                        "CursorMoved triggered for an invalid buffer",
                    );
                    return true;
                };

                let cursor = NeovimCursor::new(buffer);

                for callback in callbacks {
                    callback((&cursor, moved_by));
                }

                false
            })
        };

        // Neovim has 3 separate cursor-move-related autocommand events --
        // CursorMoved, CursorMovedI and CursorMovedC -- which are triggered
        // when the cursor is moved in Normal/Visual mode, Insert mode and in
        // the command line, respectively.
        //
        // Since ed has no concept of modes, we register the callback on both
        // CursorMoved and CursorMovedI.
        api::create_autocmd(
            ["CursorMoved", "CursorMovedI"],
            &api::opts::CreateAutocmdOpts::builder()
                .group(events.augroup_id)
                .buffer(self.0.into())
                .callback(oxi::Function::from_fn_mut(callback))
                .build(),
        )
        .expect("couldn't create autocmd on CursorMoved{I}")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}
