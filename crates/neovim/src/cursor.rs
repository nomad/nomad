//! TODO: docs.

use ed::ByteOffset;
use ed::backend::{AgentId, Buffer, Cursor};

use crate::Neovim;
use crate::buffer::{BufferId, NeovimBuffer, Point};
use crate::events::EventHandle;
use crate::oxi::api;

/// TODO: docs.
#[derive(Copy, Clone)]
pub struct NeovimCursor<'a> {
    buffer: NeovimBuffer<'a>,
}

impl<'a> NeovimCursor<'a> {
    /// TODO: docs.
    #[inline]
    pub(crate) fn new(buffer: NeovimBuffer<'a>) -> Self {
        debug_assert!(buffer.is_focused());
        Self { buffer }
    }
}

impl Cursor for NeovimCursor<'_> {
    type EventHandle = EventHandle;
    type Backend = Neovim;
    type Id = BufferId;

    #[inline]
    fn byte_offset(&self) -> ByteOffset {
        let win = api::Window::current();
        let (row, col) = win.get_cursor().expect("couldn't get cursor");
        self.buffer.byte_offset_of_point(Point {
            line_idx: row - 1,
            byte_offset: col.into(),
        })
    }

    #[inline]
    fn id(&self) -> Self::Id {
        self.buffer.id()
    }

    #[inline]
    fn on_moved<Fun>(&self, _fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&NeovimCursor<'_>, AgentId) + 'static,
    {
        todo!()
    }

    #[inline]
    fn on_removed<Fun>(&self, _fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&NeovimCursor<'_>, AgentId) + 'static,
    {
        todo!()
    }
}
