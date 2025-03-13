use crate::notify::{Notification, NotificationId};

/// TODO: docs.
pub trait Emitter {
    /// TODO: docs.
    fn emit(&mut self, notification: Notification) -> NotificationId;
}

impl<E: Emitter> Emitter for &mut E {
    #[inline]
    fn emit(&mut self, notification: Notification) -> NotificationId {
        (*self).emit(notification)
    }
}
