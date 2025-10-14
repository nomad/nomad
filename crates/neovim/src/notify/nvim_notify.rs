use core::time::Duration;

use editor::Context;
use editor::context::BorrowState;
use editor::notify::{Notification, NotificationId};
use flume::TrySendError;
use futures_util::{FutureExt, StreamExt, select_biased};
use nvim_oxi::mlua;

use crate::notify::{self, VimNotifyProvider};
use crate::{Neovim, oxi, utils};

/// Frames for the spinner animation.
pub(super) const SPINNER_FRAMES: &[char] =
    &['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'];

/// How many revolutions per minute the spinner should complete.
const SPINNER_RPM: u8 = 75;

/// How often the spinner should be updated to achieve the desired RPM.
pub(super) const SPINNER_UPDATE_INTERVAL: Duration = Duration::from_millis({
    (60_000.0 / ((SPINNER_RPM as u16 * SPINNER_FRAMES.len() as u16) as f32))
        .round() as u64
});

/// <https://github.com/rcarriga/nvim-notify>
pub struct NvimNotify;

/// TODO: docs.
pub struct NvimNotifyProgressReporter {
    notification_tx: flume::Sender<ProgressNotification>,
}

struct ProgressNotification {
    message: String,
    kind: ProgressNotificationKind,
}

#[derive(PartialEq, Eq)]
pub(super) enum ProgressNotificationKind {
    Progress,
    Success,
    Error,
}

impl NvimNotify {
    #[inline]
    pub(super) fn is_installed() -> bool {
        utils::is_module_available("notify")
    }
}

impl NvimNotifyProgressReporter {
    /// Creates a new progress reporter.
    pub fn new(ctx: &mut Context<Neovim, impl BorrowState>) -> Self {
        let (notif_tx, notif_rx) = flume::bounded::<ProgressNotification>(4);

        ctx.spawn_and_detach(async move |ctx| {
            Self::event_loop(notif_rx, ctx.namespace(), &mlua::lua()).await;
        });

        Self { notification_tx: notif_tx }
    }

    /// TODO: docs.
    pub fn report_error(self, chunks: notify::Chunks) {
        self.send_notification(ProgressNotification {
            message: chunks.concat_text(),
            kind: ProgressNotificationKind::Error,
        });
    }

    /// TODO: docs.
    pub fn report_progress(&self, chunks: notify::Chunks) {
        self.send_notification(ProgressNotification {
            message: chunks.concat_text(),
            kind: ProgressNotificationKind::Progress,
        });
    }

    /// TODO: docs.
    pub fn report_success(self, chunks: notify::Chunks) {
        self.send_notification(ProgressNotification {
            message: chunks.concat_text(),
            kind: ProgressNotificationKind::Success,
        });
    }

    async fn event_loop(
        notif_rx: flume::Receiver<ProgressNotification>,
        namespace: &editor::notify::Namespace,
        mlua: &mlua::Lua,
    ) {
        let mut spin = async_io::Timer::interval(SPINNER_UPDATE_INTERVAL);
        let mut notifications = notif_rx.into_stream();

        let Some(mut notif) = notifications.next().await else { return };
        let mut spinner_frame_idx = 0;
        let mut prev_id = None;

        let notify = notify(mlua);

        let opts = mlua
            .create_table_with_capacity(0, 4)
            .expect("failed to create options table");

        opts.raw_set("title", namespace.dot_separated().to_string())
            .expect("failed to set 'title'");

        loop {
            let hide_from_history =
                notif.kind != ProgressNotificationKind::Error;

            opts.raw_set("hide_from_history", hide_from_history)
                .expect("failed to set 'hide_from_history'");

            opts.raw_set("icon", notif.kind.icon(spinner_frame_idx))
                .expect("failed to set 'icon'");

            opts.raw_set("replace", prev_id).expect("failed to set 'replace'");

            let record = notify
                .call::<mlua::Table>((
                    &*notif.message,
                    notif.kind.log_level() as u8,
                    &opts,
                ))
                .expect("failed to call 'notify'");

            if notif.kind != ProgressNotificationKind::Progress {
                break;
            }

            prev_id = record
                .get::<mlua::Integer>("id")
                .map(Some)
                .expect("failed to get notification ID from record");

            select_biased! {
                _ = spin.next().fuse() => {
                    spinner_frame_idx += 1;
                    spinner_frame_idx %= SPINNER_FRAMES.len();
                },

                maybe_notif = notifications.next() => {
                    match maybe_notif {
                        Some(next_notif) => notif = next_notif,
                        None => break,
                    }
                },
            }
        }
    }

    fn send_notification(&self, notif: ProgressNotification) {
        if let Err(err) = self.notification_tx.try_send(notif) {
            match err {
                TrySendError::Disconnected(_) => unreachable!(),
                TrySendError::Full(_) => {},
            }
        }
    }
}

impl ProgressNotificationKind {
    pub(super) fn icon(&self, spinner_frame_idx: usize) -> char {
        match self {
            Self::Progress => SPINNER_FRAMES[spinner_frame_idx],
            Self::Success => '✔',
            Self::Error => '✘',
        }
    }

    fn log_level(&self) -> notify::Level {
        match self {
            Self::Progress | Self::Success => notify::Level::Info,
            Self::Error => notify::Level::Error,
        }
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
                // SAFETY: the object's kind is a `Dictionary`.
                oxi::ObjectKind::Dictionary => unsafe {
                    record.into_dictionary_unchecked()
                },
                _ => return None,
            };
            let id = dict.get("id")?;
            let id = match id.kind() {
                // SAFETY: the object's kind is an `Integer`.
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

/// Returns a handle to the `notify` function from the `nvim-notify` plugin.
fn notify(lua: &mlua::Lua) -> mlua::Function {
    debug_assert!(NvimNotify::is_installed());

    let require = lua
        .globals()
        .get::<mlua::Function>("require")
        .expect("'require' function not found");

    let nvim_notify = require
        .call::<mlua::Table>("notify")
        .expect("failed to require 'notify' module");

    nvim_notify
        .get::<mlua::Function>("notify")
        .expect("'notify' function not found in 'notify' module")
}
