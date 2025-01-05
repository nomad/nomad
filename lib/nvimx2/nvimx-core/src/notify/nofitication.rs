use super::{Level, Message, NotificationId};
use crate::action_ctx::ModulePath;

/// TODO: docs.
pub struct Notification<'ns> {
    /// TODO: docs.
    pub level: Level,

    /// TODO: docs.
    pub namespace: &'ns ModulePath,

    /// TODO: docs.
    pub message: Message,

    /// TODO: docs.
    pub updates_prev: Option<NotificationId>,
}
