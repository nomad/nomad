use collab::messages::{
    Deletion as CollabDeletion,
    Insertion as CollabInsertion,
    OutboundMessage,
    Session,
};
use nomad::editor::{BufferSnapshot, RemoteDeletion, RemoteInsertion};
use nomad::streams::{AppliedDeletion, AppliedEdit, AppliedInsertion};

/// Exactly the same as the [`Into`] trait, but it lets us convert `T -> U` even
/// when neither `T` nor `U` are defined in this crate.
pub(crate) trait Convert<T> {
    fn convert(self) -> T;
}

impl Convert<OutboundMessage> for AppliedEdit {
    fn convert(self) -> OutboundMessage {
        match self {
            AppliedEdit::Deletion(deletion) => deletion.convert(),
            AppliedEdit::Insertion(insertion) => insertion.convert(),
        }
    }
}

impl Convert<OutboundMessage> for AppliedInsertion {
    fn convert(self) -> OutboundMessage {
        let Self { inner, text } = self;
        OutboundMessage::LocalInsertion(CollabInsertion::new(inner, text))
    }
}

impl Convert<OutboundMessage> for AppliedDeletion {
    fn convert(self) -> OutboundMessage {
        OutboundMessage::LocalDeletion(CollabDeletion::new(self.inner))
    }
}

impl Convert<Session> for BufferSnapshot {
    fn convert(self) -> Session {
        todo!();
    }
}

impl Convert<RemoteDeletion> for CollabDeletion {
    fn convert(self) -> RemoteDeletion {
        // FIXME: don't clone.
        RemoteDeletion::new(self.crdt().clone())
    }
}

impl Convert<RemoteInsertion> for CollabInsertion {
    fn convert(self) -> RemoteInsertion {
        // FIXME: don't clone.
        RemoteInsertion::new(self.crdt().clone(), self.text().to_owned())
    }
}
