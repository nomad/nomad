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
    fn on_moved<Fun>(&self, fun: Fun) -> EventHandle
    where
        Fun: FnMut(&NeovimSelection, AgentId) + 'static,
    {
        let is_selection_alive = Shared::<bool>::new(true);
        let fun = Shared::<Fun>::new(fun);

        let cursor_moved_handle = Events::insert(
            self.buffer().events(),
            events::CursorMoved(self.buffer_id()),
            {
                let is_selection_alive = is_selection_alive.clone();
                let fun = fun.clone();
                move |(cursor, moved_by)| {
                    // Make sure the selection is still alive before calling
                    // the user's function.
                    if is_selection_alive.copied() {
                        let this = NeovimSelection::new(cursor.buffer());
                        fun.with_mut(|fun| {
                            fun(&this, moved_by);
                        })
                    }
                }
            },
        );

        let buffer_id = self.buffer_id();

        let mode_changed_handle = Events::insert(
            self.buffer().events(),
            events::ModeChanged,
            move |(buf, _old_mode, new_mode, changed_by)| {
                if buf.id() != buffer_id || !is_selection_alive.copied() {
                    return;
                }

                if new_mode.has_selected_range() {
                    let this = NeovimSelection::new(buf);
                    fun.with_mut(|fun| fun(&this, changed_by));
                } else {
                    is_selection_alive.set(false);
                }
            },
        );

        cursor_moved_handle.merge(mode_changed_handle)
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
                    && old_mode.has_selected_range()
                    // A selection is only removed if the new mode isn't also
                    // displaying a selected range.
                    && !new_mode.has_selected_range()
                {
                    fun(buffer_id, changed_by);
                }
            },
        )
    }
}
