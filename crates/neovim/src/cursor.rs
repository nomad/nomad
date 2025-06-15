//! TODO: docs.

use ed::{AgentId, Buffer, ByteOffset, Cursor, Shared};

use crate::Neovim;
use crate::buffer::{BufferId, NeovimBuffer, Point};
use crate::events::{self, EventHandle, Events};
use crate::oxi::api;

/// TODO: docs.
#[derive(Copy, Clone)]
pub struct NeovimCursor<'a> {
    buffer: NeovimBuffer<'a>,
}

impl<'a> NeovimCursor<'a> {
    /// Returns the [`NeovimBuffer`] this cursor is in.
    #[inline]
    pub(crate) fn buffer(&self) -> NeovimBuffer<'a> {
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
        Point { line_idx: row - 1, byte_offset: col.into() }
    }
}

impl Cursor for NeovimCursor<'_> {
    type Backend = Neovim;

    #[inline]
    fn buffer_id(&self) -> BufferId {
        self.buffer().id()
    }

    #[inline]
    fn byte_offset(&self) -> ByteOffset {
        self.buffer().byte_of_point(self.point())
    }

    #[inline]
    fn id(&self) -> BufferId {
        self.buffer().id()
    }

    #[track_caller]
    #[inline]
    fn r#move(&mut self, offset: ByteOffset, agent_id: AgentId) {
        debug_assert!(
            offset <= self.buffer().byte_len(),
            "offset {offset:?} is past end of buffer, length is {:?}",
            self.buffer().byte_len()
        );

        self.buffer().events().with_mut(|events| {
            let buf_id = self.buffer_id();
            if events.contains(&events::CursorMoved(buf_id)) {
                events.agent_ids.moved_cursor.insert(buf_id, agent_id);
            }
        });

        let point = self.buffer().point_of_byte(offset);

        api::Window::current()
            .set_cursor(point.line_idx + 1, point.byte_offset.into())
            .expect("couldn't set cursor");
    }

    #[inline]
    fn on_moved<Fun>(&self, fun: Fun) -> EventHandle
    where
        Fun: FnMut(&NeovimCursor, AgentId) + 'static,
    {
        let old_point = Shared::<Point>::new(self.point());
        let fun = Shared::<Fun>::new(fun);

        let cursor_moved_handle = Events::insert(
            self.buffer().events(),
            events::CursorMoved(self.buffer_id()),
            {
                let fun = fun.clone();
                let old_point = old_point.clone();
                move |(this, moved_by)| {
                    let new_point = this.point();
                    if old_point.replace(new_point) != new_point {
                        fun.with_mut(|fun| fun(this, moved_by));
                    }
                }
            },
        );

        let buffer_id = self.buffer_id();

        // The cursor position moves one character to the left when going from
        // normal to insert mode and one character to the right when going
        // from insert to normal mode with "a".
        let mode_changed_handle = Events::insert(
            self.buffer().events(),
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
        );

        cursor_moved_handle.merge(mode_changed_handle)
    }

    #[inline]
    fn on_removed<Fun>(&self, mut fun: Fun) -> EventHandle
    where
        Fun: FnMut(BufferId, AgentId) + 'static,
    {
        Events::insert(
            self.buffer().events(),
            events::BufLeave(self.buffer_id()),
            move |(&buf, unfocused_by)| fun(buf.id(), unfocused_by),
        )
    }
}
