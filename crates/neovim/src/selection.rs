//! TODO: docs.

use core::ops::{self, Range};

use editor::{AccessMut, AgentId, Buffer, ByteOffset, Selection, Shared};

use crate::buffer::{BufferId, NeovimBuffer};
use crate::events::EventHandle;
use crate::{Neovim, events};

/// TODO: docs.
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
}

impl Selection for NeovimSelection<'_> {
    type Editor = Neovim;

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
        self.buffer_id()
    }

    #[inline]
    fn on_moved<Fun>(
        &mut self,
        fun: Fun,
        nvim: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> EventHandle
    where
        Fun: FnMut(&NeovimSelection, AgentId) + 'static,
    {
        let is_selection_alive = Shared::<bool>::new(true);
        let fun = Shared::<Fun>::new(fun);

        let buffer_id = self.buffer_id();

        let cursor_moved_handle = self.events.insert(
            events::CursorMoved(buffer_id),
            {
                let is_selection_alive = is_selection_alive.clone();
                let fun = fun.clone();
                move |(cursor, moved_by)| {
                    // Make sure the selection is still alive before calling
                    // the user's function.
                    if is_selection_alive.copied() {
                        let this = NeovimSelection::new(cursor.into_buffer());
                        fun.with_mut(|fun| fun(&this, moved_by));
                    }
                }
            },
            nvim.clone(),
        );

        let mode_changed_handle = self.events.insert(
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
            nvim,
        );

        cursor_moved_handle.merge(mode_changed_handle)
    }

    #[inline]
    fn on_removed<Fun>(
        &mut self,
        mut fun: Fun,
        nvim: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> EventHandle
    where
        Fun: FnMut(BufferId, AgentId) + 'static,
    {
        let buffer_id = self.buffer_id();

        self.events.insert(
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
            nvim,
        )
    }
}

impl<'a> ops::Deref for NeovimSelection<'a> {
    type Target = NeovimBuffer<'a>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<'a> ops::DerefMut for NeovimSelection<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}
