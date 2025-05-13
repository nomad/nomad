use crate::ByteOffset;
use crate::backend::{AgentId, Backend};

/// TODO: docs.
pub trait Cursor {
    /// TODO: docs.
    type EventHandle;

    /// TODO: docs.
    type Id: Clone;

    /// TODO: docs.
    type Backend: Backend<CursorId = Self::Id>;

    /// TODO: docs.
    fn buffer_id(&self) -> <Self::Backend as Backend>::BufferId;

    /// Returns the cursor's offset in the buffer.
    fn byte_offset(&self) -> ByteOffset;

    /// Returns the cursor's ID.
    fn id(&self) -> Self::Id;

    /// Registers the given callback to be executed everytime the cursor is
    /// moved.
    ///
    /// The callback is provided with a reference to this cursor, plus the
    /// [`AgentId`] of the agent that moved it.
    fn on_moved<Fun>(&self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&<Self::Backend as Backend>::Cursor<'_>, AgentId) + 'static;

    /// Registers the given callback to be executed just before the cursor is
    /// removed.
    ///
    /// The callback is provided with a reference to this cursor, plus the
    /// [`AgentId`] of the agent that removed it.
    fn on_removed<Fun>(&self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&<Self::Backend as Backend>::Cursor<'_>, AgentId) + 'static;
}
