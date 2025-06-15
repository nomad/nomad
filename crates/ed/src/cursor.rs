use crate::{AgentId, Backend, ByteOffset};

/// TODO: docs.
pub trait Cursor {
    /// TODO: docs.
    type Backend: Backend;

    /// TODO: docs.
    fn buffer_id(&self) -> <Self::Backend as Backend>::BufferId;

    /// Returns the cursor's offset in the buffer.
    fn byte_offset(&self) -> ByteOffset;

    /// Returns the cursor's ID.
    fn id(&self) -> <Self::Backend as Backend>::CursorId;

    /// TODO: docs.
    fn r#move(&mut self, offset: ByteOffset, agent_id: AgentId);

    /// Registers the given callback to be executed everytime the cursor is
    /// moved.
    ///
    /// The callback is given a reference to this cursor, plus the [`AgentId`]
    /// of the agent that moved it.
    fn on_moved<Fun>(
        &self,
        fun: Fun,
    ) -> <Self::Backend as Backend>::EventHandle
    where
        Fun: FnMut(&<Self::Backend as Backend>::Cursor<'_>, AgentId) + 'static;

    /// Registers the given callback to be executed just before the cursor is
    /// removed.
    ///
    /// The callback is given this cursor's ID, plus the [`AgentId`] of the
    /// agent that removed it.
    fn on_removed<Fun>(
        &self,
        fun: Fun,
    ) -> <Self::Backend as Backend>::EventHandle
    where
        Fun: FnMut(<Self::Backend as Backend>::CursorId, AgentId) + 'static;
}
