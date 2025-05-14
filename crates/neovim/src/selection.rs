//! TODO: docs.

use core::ops::Range;

use ed::ByteOffset;
use ed::backend::{AgentId, Buffer, Selection};

use crate::Neovim;
use crate::buffer::{BufferId, NeovimBuffer};
use crate::events::EventHandle;

/// TODO: docs.
#[derive(Copy, Clone)]
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
    fn on_moved<Fun>(&self, _fun: Fun) -> EventHandle
    where
        Fun: FnMut(&NeovimSelection<'_>, AgentId) + 'static,
    {
        todo!()
    }

    #[inline]
    fn on_removed<Fun>(&self, _fun: Fun) -> EventHandle
    where
        Fun: FnMut(&NeovimSelection<'_>, AgentId) + 'static,
    {
        todo!()
    }
}
