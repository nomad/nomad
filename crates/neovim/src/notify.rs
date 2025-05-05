//! TODO: docs.

use ed::notify::{Emitter, Level, Notification, NotificationId};

use crate::convert::Convert;
use crate::{oxi, utils};

/// TODO: docs.
pub trait VimNotifyProvider: 'static {
    /// Converts the given `Notification` into the message that will be passed
    /// as the first argument to `vim.notify()`.
    fn to_message(&mut self, notification: &Notification) -> String;

    /// Converts the given `Notification` into the optional parameters that
    /// will be passed as the third argument to `vim.notify()`.
    fn to_opts(&mut self, notification: &Notification) -> oxi::Dictionary;

    /// Converts the return value of a call to `vim.notify()` into the
    /// [`NotificationId`] of the notification that was emitted.
    fn to_notification_id(
        &mut self,
        notify_return: oxi::Object,
    ) -> NotificationId;
}

/// TODO: docs.
pub fn detect() -> impl Into<NeovimEmitter> {
    if NvimNotify::is_installed() {
        NeovimEmitter::new(NvimNotify)
    } else {
        NeovimEmitter::new(VimNotify)
    }
}

/// TODO: docs.
pub struct NeovimEmitter {
    inner: Box<dyn VimNotifyProvider>,
}

/// https://github.com/rcarriga/nvim-notify
pub struct NvimNotify;

/// TODO: docs.
pub struct VimNotify;

impl NeovimEmitter {
    /// TODO: docs.
    #[inline]
    pub(crate) fn new<P: VimNotifyProvider>(provider: P) -> Self {
        Self { inner: Box::new(provider) }
    }
}

impl NvimNotify {
    #[inline]
    fn is_installed() -> bool {
        utils::is_module_available("notify")
    }
}

impl Emitter for NeovimEmitter {
    #[inline]
    fn emit(&mut self, notification: Notification) -> NotificationId {
        self.inner.emit(notification)
    }
}

impl Default for NeovimEmitter {
    #[inline]
    fn default() -> Self {
        Self::new(VimNotify)
    }
}

impl<T: VimNotifyProvider> From<T> for NeovimEmitter {
    #[inline]
    fn from(provider: T) -> Self {
        Self::new(provider)
    }
}

impl VimNotifyProvider for VimNotify {
    #[inline]
    fn to_message(&mut self, notification: &Notification) -> String {
        format!(
            "[{}] {}",
            notification.namespace.dot_separated(),
            notification.message.as_str()
        )
    }

    #[inline]
    fn to_opts(&mut self, _: &Notification) -> oxi::Dictionary {
        oxi::Dictionary::new()
    }

    #[inline]
    fn to_notification_id(&mut self, _: oxi::Object) -> NotificationId {
        NotificationId::new(0)
    }
}

impl VimNotifyProvider for NvimNotify {
    #[inline]
    fn to_message(&mut self, notification: &Notification) -> String {
        notification.message.as_str().to_owned()
    }

    #[inline]
    fn to_opts(&mut self, notification: &Notification) -> oxi::Dictionary {
        let mut opts = oxi::Dictionary::new();
        opts.insert(
            "title",
            notification.namespace.dot_separated().to_string(),
        );
        opts.insert(
            "replace",
            notification.updates_prev.map(|id| id.into_u64() as u32),
        );
        opts
    }

    #[inline]
    fn to_notification_id(&mut self, record: oxi::Object) -> NotificationId {
        fn inner(record: oxi::Object) -> Option<NotificationId> {
            let dict = match record.kind() {
                oxi::ObjectKind::Dictionary => unsafe {
                    record.into_dictionary_unchecked()
                },
                _ => return None,
            };
            let id = dict.get("id")?;
            let id = match id.kind() {
                oxi::ObjectKind::Integer => unsafe {
                    id.as_integer_unchecked()
                },
                _ => return None,
            };
            Some(NotificationId::new(id as u64))
        }
        inner(record).unwrap_or_else(|| NotificationId::new(0))
    }
}

impl VimNotifyProvider for Box<dyn VimNotifyProvider> {
    #[inline]
    fn to_message(&mut self, notification: &Notification) -> String {
        (**self).to_message(notification)
    }

    #[inline]
    fn to_opts(&mut self, notification: &Notification) -> oxi::Dictionary {
        (**self).to_opts(notification)
    }

    #[inline]
    fn to_notification_id(
        &mut self,
        notify_return: oxi::Object,
    ) -> NotificationId {
        (**self).to_notification_id(notify_return)
    }
}

impl Emitter for Box<dyn VimNotifyProvider> {
    #[inline]
    fn emit(&mut self, notification: Notification) -> NotificationId {
        let message = self.to_message(&notification);
        let level = notification.level.convert();
        let opts = self.to_opts(&notification);
        match oxi::api::notify(&message, level, &opts) {
            Ok(obj) => self.to_notification_id(obj),
            Err(_err) => NotificationId::new(0),
        }
    }
}

impl Convert<oxi::api::types::LogLevel> for Level {
    #[inline]
    fn convert(self) -> oxi::api::types::LogLevel {
        match self {
            Self::Off => oxi::api::types::LogLevel::Off,
            Self::Trace => oxi::api::types::LogLevel::Trace,
            Self::Debug => oxi::api::types::LogLevel::Debug,
            Self::Info => oxi::api::types::LogLevel::Info,
            Self::Warn => oxi::api::types::LogLevel::Warn,
            Self::Error => oxi::api::types::LogLevel::Error,
        }
    }
}
