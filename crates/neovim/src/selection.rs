//! TODO: docs.

use core::ops::Range;

use ed::backend::{AgentId, Buffer, Selection};
use ed::{ByteOffset, Shared};

use crate::buffer::{BufferId, NeovimBuffer};
use crate::events::{EventHandle, Events};
use crate::{Neovim, events};

/// TODO: docs.
#[derive(Clone)]
pub struct NeovimSelection<'a> {
    pub(crate) buffer: NeovimBuffer<'a>,
}

impl<'a> NeovimSelection<'a> {
    /// TODO: docs.
    #[inline]
    pub(crate) fn new(buffer: NeovimBuffer<'a>) -> Self {
        debug_assert!(buffer.selection().is_some());
        Self { buffer }
    }
}

impl Selection for NeovimSelection<'_> {
    type Backend = Neovim;

    #[inline]
    fn buffer_id(&self) -> BufferId {
        self.buffer.id()
    }

    #[inline]
    fn byte_range(&self) -> Range<ByteOffset> {
        self.buffer.selection().expect("buffer has a selection")
    }

    #[inline]
    fn id(&self) -> BufferId {
        self.buffer.id()
    }

    #[inline]
    fn on_moved<Fun>(&self, mut fun: Fun) -> EventHandle
    where
        Fun: FnMut(&NeovimSelection<'_>, AgentId) + 'static,
    {
        let is_selection_alive = Shared::<bool>::new(true);

        let cursor_moved_handle = Events::insert(
            self.buffer.events().clone(),
            events::CursorMoved(self.buffer_id()),
            {
                let is_selection_alive = is_selection_alive.clone();
                move |(cursor, moved_by)| {
                    // Make sure that the selection is still alive before
                    // calling the user's function.
                    if is_selection_alive.copied() {
                        fun(&NeovimSelection::new(cursor.buffer()), moved_by)
                    }
                }
            },
        );

        let buffer_id = self.buffer_id();

        let mode_changed_handle = Events::insert(
            self.buffer.events().clone(),
            events::ModeChanged,
            move |(buf, old_mode, new_mode, _changed_by)| {
                if buf.id() == buffer_id
                    && old_mode.is_select_or_visual()
                    // A selection is only removed if the new mode isn't also
                    // displaying a selected range.
                    && !new_mode.is_select_or_visual()
                {
                    is_selection_alive.set(false);
                }
            },
        );

        cursor_moved_handle.merge(mode_changed_handle)
    }

    #[inline]
    fn on_removed<Fun>(&self, _fun: Fun) -> EventHandle
    where
        Fun: FnMut(&NeovimSelection<'_>, AgentId) + 'static,
    {
        todo!()
    }
}
