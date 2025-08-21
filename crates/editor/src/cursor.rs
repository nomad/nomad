use crate::{AgentId, ByteOffset, Editor};

/// TODO: docs.
pub trait Cursor {
    /// TODO: docs.
    type Editor: Editor;

    /// TODO: docs.
    fn buffer_id(&self) -> <Self::Editor as Editor>::BufferId;

    /// Returns the cursor's offset in the buffer.
    fn byte_offset(&self) -> ByteOffset;

    /// Returns the cursor's ID.
    fn id(&self) -> <Self::Editor as Editor>::CursorId;

    /// TODO: docs.
    fn r#move(&mut self, offset: ByteOffset, agent_id: AgentId);

    /// Registers the given callback to be executed every time the cursor is
    /// moved.
    ///
    /// The callback is given a reference to this cursor, plus the [`AgentId`]
    /// of the agent that moved it.
    fn on_moved<Fun>(&self, fun: Fun) -> <Self::Editor as Editor>::EventHandle
    where
        Fun: FnMut(&<Self::Editor as Editor>::Cursor<'_>, AgentId) + 'static;

    /// Registers the given callback to be executed just before the cursor is
    /// removed.
    ///
    /// The callback is given this cursor's ID, plus the [`AgentId`] of the
    /// agent that removed it.
    fn on_removed<Fun>(
        &self,
        fun: Fun,
    ) -> <Self::Editor as Editor>::EventHandle
    where
        Fun: FnMut(<Self::Editor as Editor>::CursorId, AgentId) + 'static;
}
