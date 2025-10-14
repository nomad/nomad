use core::time::Duration;
use core::{fmt, iter};
use std::borrow::Cow;

use editor::Context;
use editor::context::BorrowState;
use flume::TrySendError;
use futures_util::{FutureExt, StreamExt, select_biased};
use nvim_oxi::api;

use crate::notify::nvim_notify::{
    ProgressNotificationKind,
    SPINNER_FRAMES,
    SPINNER_UPDATE_INTERVAL,
};
use crate::{Neovim, notify};

/// TODO: docs.
pub struct NvimEcho {}

/// TODO: docs.
pub struct NvimEchoProgressReporter {
    notification_tx: flume::Sender<ProgressNotification>,
}

struct ProgressNotification {
    chunks: notify::Chunks,
    kind: ProgressNotificationKind,
}

/// The chunks given to `nvim_echo`.
struct NvimEchoChunks<'ns> {
    title: Title<'ns>,
    message_chunks: notify::Chunks,
}

struct Title<'ns> {
    icon: char,
    namespace: &'ns editor::notify::Namespace,
    hl_group: Option<&'static str>,
}

impl NvimEchoProgressReporter {
    /// Creates a new progress reporter.
    pub fn new(ctx: &mut Context<Neovim, impl BorrowState>) -> Self {
        let (notif_tx, notif_rx) = flume::bounded::<ProgressNotification>(4);

        ctx.spawn_and_detach(async move |ctx| {
            Self::event_loop(notif_rx, ctx.namespace()).await;
        });

        Self { notification_tx: notif_tx }
    }

    /// TODO: docs.
    pub fn report_error(self, chunks: notify::Chunks) {
        self.send_notification(ProgressNotification {
            chunks,
            kind: ProgressNotificationKind::Error,
        });
    }

    /// TODO: docs.
    pub fn report_progress(&self, chunks: notify::Chunks) {
        self.send_notification(ProgressNotification {
            chunks,
            kind: ProgressNotificationKind::Progress,
        });
    }

    /// TODO: docs.
    pub fn report_success(self, chunks: notify::Chunks) {
        self.send_notification(ProgressNotification {
            chunks,
            kind: ProgressNotificationKind::Success,
        });
    }

    fn clear_message_area() {
        api::echo([("", None::<&str>)], false, &Default::default())
            .expect("couldn't clear message area");
    }

    async fn event_loop(
        notif_rx: flume::Receiver<ProgressNotification>,
        namespace: &editor::notify::Namespace,
    ) {
        let mut spin = async_io::Timer::interval(SPINNER_UPDATE_INTERVAL);
        let mut notifications = notif_rx.into_stream();
        let mut spinner_frame_idx = 0;

        let opts = api::opts::EchoOpts::default();

        let Some(first_notif) = notifications.next().await else { return };

        let mut chunks = NvimEchoChunks {
            title: Title {
                icon: first_notif.kind.icon(spinner_frame_idx),
                hl_group: Some(first_notif.kind.hl_group()),
                namespace,
            },
            message_chunks: first_notif.chunks,
        };

        let initial_cmdheight = Self::get_cmdheight();
        let mut current_cmdheight = initial_cmdheight;
        let mut last_notif_kind = first_notif.kind;

        loop {
            let add_to_history =
                last_notif_kind == ProgressNotificationKind::Error;

            api::echo(chunks.to_iter(), add_to_history, &opts)
                .expect("couldn't echo progress message");

            if last_notif_kind != ProgressNotificationKind::Progress {
                let wait_duration =
                    Duration::from_millis(match last_notif_kind {
                        ProgressNotificationKind::Success => 2500,
                        ProgressNotificationKind::Error => 3500,
                        ProgressNotificationKind::Progress => unreachable!(),
                    });

                // Wait a bit before clearing the final message to give the
                // user a chance to read it.
                async_io::Timer::after(wait_duration).await;

                Self::clear_message_area();

                break;
            }

            select_biased! {
                _ = spin.next().fuse() => {
                    spinner_frame_idx += 1;
                    spinner_frame_idx %= SPINNER_FRAMES.len();
                    chunks.title.icon = SPINNER_FRAMES[spinner_frame_idx];
                },

                maybe_notif = notifications.next() => {
                    let Some(notif) = maybe_notif else { break };

                    chunks.title.icon = notif.kind.icon(spinner_frame_idx);
                    chunks.title.hl_group = Some(notif.kind.hl_group());
                    chunks.message_chunks = notif.chunks;
                    last_notif_kind = notif.kind;

                    // We need to increase the cmdheight if its current value
                    // is smaller than what's needed to fully render the
                    // message.
                    let min_cmdheight = chunks.num_lines();
                    if current_cmdheight < min_cmdheight {
                        Self::set_cmdheight(min_cmdheight);
                        current_cmdheight = min_cmdheight;
                    }
                },
            }
        }

        // Reset the cmdheight to its initial value.
        if current_cmdheight != initial_cmdheight {
            Self::set_cmdheight(initial_cmdheight);
        }
    }

    /// Returns the current value of `cmdheight`.
    fn get_cmdheight() -> u16 {
        api::get_option_value("cmdheight", &Default::default())
            .expect("couldn't get 'cmdheight'")
    }

    fn send_notification(&self, notif: ProgressNotification) {
        if let Err(err) = self.notification_tx.try_send(notif) {
            match err {
                TrySendError::Disconnected(_) => unreachable!(),
                TrySendError::Full(_) => {},
            }
        }
    }

    /// Sets the `cmdheight` option to the given value.
    fn set_cmdheight(cmdheight: u16) {
        api::set_option_value("cmdheight", cmdheight, &Default::default())
            .expect("couldn't set 'cmdheight'")
    }
}

impl ProgressNotificationKind {
    fn hl_group(&self) -> &'static str {
        match self {
            Self::Progress => "DiagnosticInfo",
            Self::Success => "DiagnosticOk",
            Self::Error => "DiagnosticError",
        }
    }
}
impl NvimEchoChunks<'_> {
    fn num_lines(&self) -> u16 {
        1 + !self.message_chunks.is_empty() as u16
            + self
                .message_chunks
                .iter()
                .map(|chunk| {
                    memchr::memchr_iter(b'\n', chunk.text().as_bytes()).count()
                        as u16
                })
                .sum::<u16>()
    }

    fn to_iter(&self) -> impl Iterator<Item = (Cow<'_, str>, Option<&str>)> {
        iter::once((Cow::Owned(self.title.to_string()), self.title.hl_group))
            .chain(iter::once((Cow::Borrowed("\n"), None::<&str>)))
            .chain(
                self.message_chunks.iter().map(|chunk| {
                    (Cow::Borrowed(chunk.text()), chunk.hl_group())
                }),
            )
    }
}

impl fmt::Display for Title<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.icon, self.namespace.dot_separated())
    }
}
