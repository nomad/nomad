use crate::notify::{Level, Message, NotificationId, Source};

/// TODO: docs.
pub struct Notification<'src> {
    /// TODO: docs.
    pub level: Level,

    /// TODO: docs.
    pub message: Message,

    /// TODO: docs.
    pub source: Source<'src>,

    /// TODO: docs.
    pub updates_prev: Option<NotificationId>,
}
