use nohash::IntMap as NoHashMap;

use crate::buffer_id::BufferId;
use crate::ActorId;

#[derive(Default)]
pub(crate) struct ActorMap {
    /// Map from [`BufferId`] to the [`ActorId`] that last added it.
    buffer_addition: NoHashMap<BufferId, ActorId>,

    /// Map from [`BufferId`] to the [`ActorId`] that last focused it.
    buffer_focus: NoHashMap<BufferId, ActorId>,

    /// Map from [`BufferId`] to the [`ActorId`] that last edited it.
    edit: NoHashMap<BufferId, ActorId>,
}

impl ActorMap {
    /// Registers the given [`ActorId`] as the last one to add the given
    /// buffer.
    pub(crate) fn added_buffer(
        &mut self,
        buffer_id: BufferId,
        actor_id: ActorId,
    ) {
        self.buffer_addition.insert(buffer_id, actor_id);
    }

    /// Registers the given [`ActorId`] as the last one to edit the given
    /// buffer.
    pub(crate) fn edited_buffer(
        &mut self,
        buffer_id: BufferId,
        actor_id: ActorId,
    ) {
        self.edit.insert(buffer_id, actor_id);
    }

    /// Registers the given [`ActorId`] as the last one to focus the given
    /// buffer.
    pub(crate) fn focused_buffer(
        &mut self,
        buffer_id: BufferId,
        actor_id: ActorId,
    ) {
        self.buffer_focus.insert(buffer_id, actor_id);
    }

    /// Registers the given [`ActorId`] as the last one to move the cursor in
    /// the given buffer.
    pub(crate) fn moved_cursor(
        &mut self,
        buffer_id: BufferId,
        actor_id: ActorId,
    ) {
        self.buffer_focus.insert(buffer_id, actor_id);
    }

    /// Removes the [`ActorId`] that last added the given buffer.
    pub(crate) fn take_added_buffer(
        &mut self,
        buffer_id: &BufferId,
    ) -> ActorId {
        self.buffer_addition.remove(buffer_id).unwrap_or(ActorId::unknown())
    }

    /// Removes the [`ActorId`] that last edited the given buffer.
    pub(crate) fn take_edited_buffer(
        &mut self,
        buffer_id: &BufferId,
    ) -> ActorId {
        self.edit.remove(buffer_id).unwrap_or(ActorId::unknown())
    }

    /// Removes the [`ActorId`] that last focused the given buffer.
    pub(crate) fn take_focused_buffer(
        &mut self,
        buffer_id: &BufferId,
    ) -> ActorId {
        self.buffer_focus.remove(buffer_id).unwrap_or(ActorId::unknown())
    }

    /// Removes the [`ActorId`] that last moved the cursor in the given buffer.
    pub(crate) fn take_moved_cursor(
        &mut self,
        buffer_id: &BufferId,
    ) -> ActorId {
        self.buffer_focus.remove(buffer_id).unwrap_or(ActorId::unknown())
    }
}
