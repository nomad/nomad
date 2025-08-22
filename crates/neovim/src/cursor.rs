//! TODO: docs.

use core::ops;

use editor::{AccessMut, AgentId, Buffer, ByteOffset, Cursor, Shared};

use crate::Neovim;
use crate::buffer::{BufferId, NeovimBuffer, Point};
use crate::events::{self, EventHandle};
use crate::oxi::api;

/// TODO: docs.
pub struct NeovimCursor<'a> {
    buffer: NeovimBuffer<'a>,
}

impl<'a> NeovimCursor<'a> {
    #[inline]
    pub(crate) fn into_buffer(self) -> NeovimBuffer<'a> {
        self.buffer
    }

    #[inline]
    pub(crate) fn new(buffer: NeovimBuffer<'a>) -> Self {
        debug_assert!(buffer.is_focused());
        Self { buffer }
    }

    /// Returns the [`Point`] this cursor is currently at.
    #[inline]
    pub(crate) fn point(&self) -> Point {
        let (row, col) =
            api::Window::current().get_cursor().expect("couldn't get cursor");
        Point { line_idx: row - 1, byte_offset: col }
    }

    #[inline]
    pub(crate) fn reborrow(&mut self) -> NeovimCursor<'_> {
        NeovimCursor { buffer: self.buffer.reborrow() }
    }
}

impl Cursor for NeovimCursor<'_> {
    type Editor = Neovim;

    #[inline]
    fn buffer_id(&self) -> BufferId {
        self.buffer.id()
    }

    #[inline]
    fn byte_offset(&self) -> ByteOffset {
        self.buffer.byte_of_point(self.point())
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
        Fun: FnMut(&NeovimCursor, AgentId) + 'static,
    {
        let old_point = Shared::<Point>::new(self.point());
        let fun = Shared::<Fun>::new(fun);

        let buffer_id = self.buffer_id();

        let cursor_moved_handle = self.events.insert(
            events::CursorMoved(buffer_id),
            {
                let fun = fun.clone();
                let old_point = old_point.clone();
                move |(this, moved_by)| {
                    let new_point = this.point();
                    if old_point.replace(new_point) != new_point {
                        fun.with_mut(|fun| fun(&this, moved_by));
                    }
                }
            },
            nvim.clone(),
        );

        // The cursor position moves one character to the left when going from
        // normal to insert mode and one character to the right when going
        // from insert to normal mode with "a".
        let mode_changed_handle = self.events.insert(
            events::ModeChanged,
            move |(buf, old_mode, new_mode, changed_by)| {
                if buf.id() == buffer_id
                    && (old_mode.is_insert() || new_mode.is_insert())
                {
                    let this = NeovimCursor::new(buf);
                    let new_point = this.point();
                    if old_point.replace(new_point) != new_point {
                        fun.with_mut(|fun| fun(&this, changed_by));
                    }
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
            events::BufLeave(buffer_id),
            move |(buf, unfocused_by)| fun(buf.id(), unfocused_by),
            nvim,
        )
    }

    #[track_caller]
    #[inline]
    fn schedule_move(&mut self, offset: ByteOffset, agent_id: AgentId) {
        debug_assert!(
            offset <= self.buffer.byte_len(),
            "offset {offset:?} is past end of buffer, length is {:?}",
            self.buffer.byte_len()
        );

        let buffer_id = self.buffer_id();

        if self.events.contains(&events::CursorMoved(buffer_id)) {
            self.events.agent_ids.moved_cursor.insert(buffer_id, agent_id);
        }

        let point = self.buffer.point_of_byte(offset);

        api::Window::current()
            .set_cursor(point.line_idx + 1, point.byte_offset)
            .expect("couldn't set cursor");
    }
}

impl<'a> ops::Deref for NeovimCursor<'a> {
    type Target = NeovimBuffer<'a>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<'a> ops::DerefMut for NeovimCursor<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}
