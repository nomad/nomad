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

#[cfg(not(feature = "nightly"))]
type OptsOrMessageId = api::opts::EchoOpts;

#[cfg(feature = "nightly")]
type OptsOrMessageId = Option<api::types::EchoMessageId>;

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

enum EventLoopOutput {
    Success,
    Error,
    Cancelled,
}

impl NvimEchoProgressReporter {
    /// Creates a new progress reporter.
    pub fn new(ctx: &mut Context<Neovim, impl BorrowState>) -> Self {
        let (notif_tx, notif_rx) = flume::bounded::<ProgressNotification>(4);

        ctx.spawn_and_detach(async move |ctx| {
            let initial_cmdheight = Self::get_cmdheight();

            let mut current_cmdheight = initial_cmdheight;

            let output = Self::event_loop(
                notif_rx,
                ctx.namespace(),
                &mut current_cmdheight,
            )
            .await;

            let wait_duration = Duration::from_millis(match output {
                EventLoopOutput::Success => 2500,
                EventLoopOutput::Error => 3500,
                EventLoopOutput::Cancelled => 0,
            });

            // Wait a bit before clearing the final message to give the user a
            // chance to read it.
            async_io::Timer::after(wait_duration).await;

            // Also wait to mess with the message area if the user is currently
            // interacting with it (e.g. they're being show the "Press ENTER"
            // prompt, the "-- more --" prompt, etc).
            while api::get_mode().mode.as_bytes().first() == Some(&b'r') {
                async_io::Timer::after(Duration::from_millis(100)).await;
            }

            Self::clear_message_area();

            // Reset the cmdheight to its initial value.
            if current_cmdheight != initial_cmdheight {
                Self::set_cmdheight(initial_cmdheight);
            }
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

    fn echo(
        chunks: &NvimEchoChunks<'_>,
        notif_kind: ProgressNotificationKind,
        opts_or_message_id: &mut OptsOrMessageId,
    ) {
        let add_to_history = notif_kind == ProgressNotificationKind::Error;

        #[cfg(not(feature = "nightly"))]
        let opts = opts_or_message_id;

        #[cfg(feature = "nightly")]
        let opts = {
            let mut builder = api::opts::EchoOpts::builder();

            if let Some(message_id) = opts_or_message_id.take() {
                builder.id(message_id);
            }

            builder
                .kind("progress")
                .title(chunks.title.to_string())
                .status(notif_kind.into())
                .build()
        };

        #[cfg(feature = "nightly")]
        let opts = &opts;

        let _message_id = api::echo(chunks.to_iter(), add_to_history, opts)
            .expect("couldn't echo progress message");

        #[cfg(feature = "nightly")]
        {
            *opts_or_message_id = Some(_message_id);
        }
    }

    async fn event_loop(
        notif_rx: flume::Receiver<ProgressNotification>,
        namespace: &editor::notify::Namespace,
        current_cmdheight: &mut u16,
    ) -> EventLoopOutput {
        let Ok(first_notif) = notif_rx.recv_async().await else {
            return EventLoopOutput::Cancelled;
        };

        let mut spin = async_io::Timer::interval(SPINNER_UPDATE_INTERVAL);
        let mut spinner_frame_idx = 0;
        let mut opts_or_msg_id = OptsOrMessageId::default();

        let mut chunks = NvimEchoChunks {
            title: Title {
                icon: first_notif.kind.icon(spinner_frame_idx),
                hl_group: Some(first_notif.kind.hl_group()),
                namespace,
            },
            message_chunks: first_notif.chunks,
        };

        let mut last_notif_kind = first_notif.kind;

        loop {
            // We need to increase the cmdheight if its current value is
            // smaller than what's needed to fully render the message.
            let min_cmdheight = chunks.num_lines();
            if *current_cmdheight < min_cmdheight {
                Self::set_cmdheight(min_cmdheight);
                *current_cmdheight = min_cmdheight;
            }

            Self::echo(&chunks, last_notif_kind, &mut opts_or_msg_id);

            match last_notif_kind {
                ProgressNotificationKind::Success => {
                    return EventLoopOutput::Success;
                },
                ProgressNotificationKind::Error => {
                    return EventLoopOutput::Error;
                },
                ProgressNotificationKind::Progress => {},
            }

            'spin: loop {
                select_biased! {
                    _ = spin.next().fuse() => {
                        spinner_frame_idx += 1;
                        spinner_frame_idx %= SPINNER_FRAMES.len();
                        chunks.title.icon = SPINNER_FRAMES[spinner_frame_idx];
                        Self::echo(&chunks, last_notif_kind, &mut opts_or_msg_id);
                        continue 'spin;
                    },

                    maybe_notif = notif_rx.recv_async() => {
                        let Ok(notif) = maybe_notif else {
                            return EventLoopOutput::Cancelled;
                        };
                        chunks.title.icon = notif.kind.icon(spinner_frame_idx);
                        chunks.title.hl_group = Some(notif.kind.hl_group());
                        chunks.message_chunks = notif.chunks;
                        last_notif_kind = notif.kind;
                        break 'spin;
                    },
                }
            }
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
    fn hl_group(self) -> &'static str {
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
        #[cfg(not(feature = "nightly"))]
        let title = iter::once((
            Cow::Owned(self.title.to_string()),
            self.title.hl_group,
        ));

        // On Nightly the title is given to `nvim_echo` via the opts, so we
        // don't need to include it in the iterator.
        #[cfg(feature = "nightly")]
        let title = iter::empty();

        title.chain(iter::once((Cow::Borrowed("\n"), None::<&str>))).chain(
            self.message_chunks
                .iter()
                .map(|chunk| (Cow::Borrowed(chunk.text()), chunk.hl_group())),
        )
    }
}

impl fmt::Display for Title<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.icon, self.namespace.dot_separated())
    }
}
