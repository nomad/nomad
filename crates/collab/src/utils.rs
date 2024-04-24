use collab_client::messages::{
    Deletion as CollabDeletion,
    File,
    Insertion as CollabInsertion,
    OutboundMessage,
    PeerId,
    Project,
    Session,
};
use nomad::streams::{
    AppliedDeletion,
    AppliedEdit,
    AppliedEditKind,
    AppliedInsertion,
};
use nomad::{BufferSnapshot, Edit};

/// Exactly the same as the [`Into`] trait, but it lets us convert `T -> U`
/// even when neither `T` nor `U` are defined in this crate.
pub(crate) trait Convert<T> {
    fn convert(self) -> T;
}

impl Convert<OutboundMessage> for Edit {
    fn convert(self) -> OutboundMessage {
        todo!();
    }
}

impl Convert<OutboundMessage> for AppliedEdit {
    fn convert(self) -> OutboundMessage {
        match self.into_kind() {
            AppliedEditKind::Deletion(deletion) => deletion.convert(),
            AppliedEditKind::Insertion(insertion) => insertion.convert(),
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
        let file = File::build_document()
            .file_id(unsafe { core::mem::transmute(0u64) })
            .name("Untitled")
            .replica(self.replica())
            .text(self.text().to_string())
            .build();

        let project = Project::builder().root(file).build();

        let peers = vec![PeerId::new(self.replica().id())];

        Session::new(project, peers)
    }
}
