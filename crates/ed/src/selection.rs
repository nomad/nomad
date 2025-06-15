use core::ops::Range;

use crate::{AgentId, ByteOffset, Editor};

/// TODO: docs.
pub trait Selection {
    /// TODO: docs.
    type Editor: Editor;

    /// TODO: docs.
    fn buffer_id(&self) -> <Self::Editor as Editor>::BufferId;

    /// Returns the selection's byte range in the buffer.
    fn byte_range(&self) -> Range<ByteOffset>;

    /// Returns the selection's ID.
    fn id(&self) -> <Self::Editor as Editor>::SelectionId;

    /// Registers the given callback to be executed everytime the selection is
    /// moved, i.e. every time its start or end offset is changed.
    ///
    /// The callback is given a reference to this selection, plus the
    /// [`AgentId`] of the agent that moved it.
    fn on_moved<Fun>(&self, fun: Fun) -> <Self::Editor as Editor>::EventHandle
    where
        Fun:
            FnMut(&<Self::Editor as Editor>::Selection<'_>, AgentId) + 'static;

    /// Registers the given callback to be executed just before the selection
    /// is removed.
    ///
    /// The callback is given this selection's ID, plus the [`AgentId`] of the
    /// agent that removed it.
    fn on_removed<Fun>(
        &self,
        fun: Fun,
    ) -> <Self::Editor as Editor>::EventHandle
    where
        Fun: FnMut(<Self::Editor as Editor>::SelectionId, AgentId) + 'static;
}
