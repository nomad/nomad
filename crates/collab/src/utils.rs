use collab_client::messages::{
    File,
    FileId,
    OutboundMessage,
    PeerId,
    Project,
    Session,
};
use nomad::prelude::{BufferSnapshot, Edit};

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

impl Convert<Session> for BufferSnapshot {
    fn convert(self) -> Session {
        let file = File::build_document()
            // TODO: don't transmute.
            .file_id(unsafe { core::mem::transmute::<u64, FileId>(0u64) })
            .name("Untitled")
            .replica(self.replica())
            .text(self.text().to_string())
            .build();

        let project = Project::builder().root(file).build();

        let peers = vec![PeerId::new(self.replica().id())];

        Session::new(project, peers)
    }
}
