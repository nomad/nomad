use crate::fs::AbsPathBuf;

/// TODO: docs.
#[derive(Debug, Clone)]
pub struct FsEvent<Ts> {
    /// TODO: docs.
    pub kind: FsEventKind,

    /// TODO: docs.
    pub path: AbsPathBuf,

    /// TODO: docs.
    pub timestamp: Ts,
}

/// TODO: docs.
#[derive(Debug, Clone)]
pub enum FsEventKind {
    /// TODO: docs.
    CreatedDir,

    /// TODO: docs.
    CreatedFile,

    /// TODO: docs.
    DeletedDir,

    /// TODO: docs.
    DeletedFile,

    /// TODO: docs.
    ModifiedFile,

    /// TODO: docs.
    RenamedNode(AbsPathBuf),
}

#[cfg(feature = "os-fs")]
mod os_fs {
    use notify::EventKind;
    use notify::event::{CreateKind, ModifyKind, RemoveKind, RenameMode};
    use smallvec::SmallVec;

    use super::*;
    use crate::fs;

    impl FsEvent<<fs::os::OsFs as fs::Fs>::Timestamp> {
        pub(crate) fn from_notify(
            event: notify::Event,
            timestamp: <fs::os::OsFs as fs::Fs>::Timestamp,
        ) -> SmallVec<[Self; 1]> {
            let mut events = SmallVec::new();

            let mut paths = event
                .paths
                .into_iter()
                .filter_map(|path| AbsPathBuf::try_from(path).ok());

            let kind = match event.kind {
                EventKind::Create(kind) => match kind {
                    CreateKind::File => FsEventKind::CreatedFile,
                    CreateKind::Folder => FsEventKind::CreatedDir,
                    _ => return events,
                },
                EventKind::Modify(kind) => match kind {
                    ModifyKind::Data(_) => FsEventKind::ModifiedFile,
                    ModifyKind::Name(RenameMode::Both) => {
                        let Some(from) = paths.next() else { return events };
                        let Some(to) = paths.next() else { return events };
                        let event = Self {
                            kind: FsEventKind::RenamedNode(to),
                            path: from,
                            timestamp,
                        };
                        events.push(event);
                        return events;
                    },
                    _ => return events,
                },
                EventKind::Remove(kind) => match kind {
                    RemoveKind::File => FsEventKind::DeletedFile,
                    RemoveKind::Folder => FsEventKind::DeletedDir,
                    _ => return events,
                },
                _ => return events,
            };

            events.extend(paths.map(|path| Self {
                kind: kind.clone(),
                path,
                timestamp,
            }));

            events
        }
    }
}
