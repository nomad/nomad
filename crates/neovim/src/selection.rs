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
    buffer: NeovimBuffer<'a>,
}

impl<'a> NeovimSelection<'a> {
    /// TODO: docs.
    #[inline]
    pub(crate) fn new(buffer: NeovimBuffer<'a>) -> Self {
        debug_assert!(buffer.selection().is_some());
        Self { buffer }
    }

    /// Returns the [`NeovimBuffer`] this selection is in.
    #[inline]
    pub(crate) fn buffer(&self) -> NeovimBuffer<'a> {
        self.buffer
    }
}

impl Selection for NeovimSelection<'_> {
    type Backend = Neovim;

    #[inline]
    fn buffer_id(&self) -> BufferId {
        self.buffer().id()
    }

    #[inline]
    fn byte_range(&self) -> Range<ByteOffset> {
        self.buffer().selection().expect("buffer has a selection")
    }

    #[inline]
    fn id(&self) -> BufferId {
        self.buffer().id()
    }

    #[inline]
    fn on_moved<Fun>(&self, mut fun: Fun) -> EventHandle
    where
        Fun: FnMut(&NeovimSelection<'_>, AgentId) + 'static,
    {
        let is_selection_alive = Shared::<bool>::new(true);

        let selection_removed_handle = self.on_removed({
            let is_selection_alive = is_selection_alive.clone();
            move |_buf_id, _removed_by| is_selection_alive.set(false)
        });

        let cursor_moved_handle = Events::insert(
            self.buffer().events(),
            events::CursorMoved(self.buffer_id()),
            move |(cursor, moved_by)| {
                // Make sure the selection is still alive before calling the
                // user's function.
                if is_selection_alive.copied() {
                    fun(&NeovimSelection::new(cursor.buffer()), moved_by)
                }
            },
        );

        cursor_moved_handle.merge(selection_removed_handle)
    }

    #[inline]
    fn on_removed<Fun>(&self, mut fun: Fun) -> EventHandle
    where
        Fun: FnMut(BufferId, AgentId) + 'static,
    {
        let buffer_id = self.buffer_id();

        Events::insert(
            self.buffer().events(),
            events::ModeChanged,
            move |(buf, old_mode, new_mode, changed_by)| {
                if buf.id() == buffer_id
                    && old_mode.is_select_or_visual()
                    // A selection is only removed if the new mode isn't also
                    // displaying a selected range.
                    && !new_mode.is_select_or_visual()
                {
                    fun(buffer_id, changed_by);
                }
            },
        )
    }
}
