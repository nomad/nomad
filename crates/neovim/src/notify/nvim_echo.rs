use core::cell::Cell;
use core::time::Duration;
use core::{fmt, iter};

use compact_str::{CompactString, ToCompactString};
use executor::LocalSpawner;
use flume::TrySendError;
use futures_util::{FutureExt, StreamExt, select_biased};
use nvim_oxi::api;

use crate::executor::NeovimLocalSpawner;
use crate::notify;
use crate::notify::nvim_notify::{
    Icon,
    SPINNER_FRAMES,
    SPINNER_UPDATE_INTERVAL,
};
use crate::notify::progress_reporter::{
    ProgressNotification,
    ProgressNotificationKind,
};

#[cfg(not(feature = "nightly"))]
type OptsOrMessageId = api::opts::EchoOpts;

#[cfg(feature = "nightly")]
type OptsOrMessageId = Option<api::types::EchoMessageId>;

thread_local! {
    /// The initial value of the `cmdheight` option before `NvimEcho` and
    /// `NvimEchoProgressReporter` mess with it.
    static INITIAL_CMDHEIGHT: Cell<Option<u16>> = const { Cell::new(None) };
}

/// TODO: docs.
pub struct NvimEcho {}

/// TODO: docs.
pub struct NvimEchoProgressReporter {
    notification_tx: flume::Sender<ProgressNotification>,
}

/// The chunks given to `nvim_echo`.
struct NvimEchoChunks<'ns> {
    title: Title<'ns>,
    message_chunks: notify::Chunks,
}

struct Title<'ns> {
    icon: Icon,
    namespace: &'ns editor::notify::Namespace,
    hl_group: Option<&'static str>,
}

enum EventLoopOutput {
    Success,
    Error,
    Cancelled,
}

impl NvimEcho {
    pub(crate) fn notify(
        namespace: &editor::notify::Namespace,
        message_chunks: notify::Chunks,
        level: notify::Level,
        spawner: &mut NeovimLocalSpawner,
    ) {
        let (icon, hl_group) = match level {
            notify::Level::Trace => ('🔍', None),
            notify::Level::Debug => ('🐛', None),
            notify::Level::Info => ('ℹ', Some("DiagnosticInfo")),
            notify::Level::Warn => ('⚠', Some("DiagnosticWarn")),
            notify::Level::Error => ('✘', Some("DiagnosticError")),
            _ => return,
        };

        let chunks = NvimEchoChunks {
            title: Title { icon: Icon::Char(icon), hl_group, namespace },
            message_chunks,
        };

        let initial_cmdheight = Self::get_initial_cmdheight();

        if chunks.num_lines() > initial_cmdheight {
            Self::set_cmdheight(chunks.num_lines());
        }

        api::echo(chunks.to_iter(true), true, &Default::default())
            .expect("couldn't echo notification message");

        spawner
            .spawn(async move {
                Self::clear_message_area(Duration::from_millis(match level {
                    notify::Level::Error => 3500,
                    _ => 2500,
                }))
                .await;

                Self::restore_cmdheight(initial_cmdheight);
            })
            .detach();
    }

    /// Clears the message area after the given duration.
    async fn clear_message_area(wait: Duration) {
        async_io::Timer::after(wait).await;

        // Wait to mess with the message area if the user is currently
        // interacting with it (e.g. they're being show the "Press ENTER"
        // prompt, the "-- more --" prompt, etc).
        while api::get_mode().mode.as_bytes().first() == Some(&b'r') {
            async_io::Timer::after(Duration::from_millis(100)).await;
        }

        api::echo([("", None::<&str>)], false, &Default::default())
            .expect("couldn't clear message area");
    }

    /// Returns the current value of `cmdheight`.
    fn get_cmdheight() -> u16 {
        api::get_option_value("cmdheight", &Default::default())
            .expect("couldn't get 'cmdheight'")
    }

    /// Returns the value of [`INITIAL_CMDHEIGHT`], setting it to the current
    /// `cmdheight` if it's not already set.
    fn get_initial_cmdheight() -> u16 {
        INITIAL_CMDHEIGHT.with(|cell| match cell.get() {
            Some(cmdheight) => cmdheight,
            None => {
                let cmdheight = Self::get_cmdheight();
                cell.set(Some(cmdheight));
                cmdheight
            },
        })
    }

    /// Restores the `cmdheight` option to the given value, also resetting
    /// [`INITIAL_CMDHEIGHT`] to `None`.
    fn restore_cmdheight(initial_cmdheight: u16) {
        Self::set_cmdheight(initial_cmdheight);
        INITIAL_CMDHEIGHT.with(|cell| cell.set(None));
    }

    /// Sets the `cmdheight` option to the given value.
    fn set_cmdheight(cmdheight: u16) {
        api::set_option_value("cmdheight", cmdheight, &Default::default())
            .expect("couldn't set 'cmdheight'")
    }
}

impl NvimEchoProgressReporter {
    /// Creates a new progress reporter.
    pub fn new(
        namespace: editor::notify::Namespace,
        spawner: &mut NeovimLocalSpawner,
    ) -> Self {
        let (notif_tx, notif_rx) = flume::bounded::<ProgressNotification>(4);

        spawner
            .spawn(async move {
                let initial_cmdheight = NvimEcho::get_initial_cmdheight();

                let mut current_cmdheight = initial_cmdheight;

                let output = Self::event_loop(
                    notif_rx,
                    &namespace,
                    &mut current_cmdheight,
                )
                .await;

                NvimEcho::clear_message_area(Duration::from_millis(
                    match output {
                        EventLoopOutput::Success => 2500,
                        EventLoopOutput::Error => 3500,
                        EventLoopOutput::Cancelled => 0,
                    },
                ))
                .await;

                // Reset the cmdheight to its initial value.
                if current_cmdheight != initial_cmdheight {
                    NvimEcho::restore_cmdheight(initial_cmdheight);
                }
            })
            .detach();

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
    pub fn report_progress(
        &self,
        chunks: notify::Chunks,
        perc: Option<notify::Percentage>,
    ) {
        self.send_notification(ProgressNotification {
            chunks,
            kind: ProgressNotificationKind::Progress(perc),
        });
    }

    /// TODO: docs.
    pub fn report_success(self, chunks: notify::Chunks) {
        self.send_notification(ProgressNotification {
            chunks,
            kind: ProgressNotificationKind::Success,
        });
    }

    pub(super) fn send_notification(&self, notif: ProgressNotification) {
        if let Err(err) = self.notification_tx.try_send(notif) {
            match err {
                TrySendError::Disconnected(_) => unreachable!(),
                TrySendError::Full(_) => {},
            }
        }
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

            if let ProgressNotificationKind::Progress(Some(perc)) = notif_kind
            {
                builder.percent(perc);
            }

            builder
                .kind("progress")
                .title(&*chunks.title.to_compact_string())
                .status(notif_kind.into())
                .build()
        };

        #[cfg(feature = "nightly")]
        let opts = &opts;

        // On Nightly the title is set in the opts, so we don't need to include
        // it in the iterator.
        let include_title = cfg!(not(feature = "nightly"));

        #[cfg_attr(not(feature = "nightly"), expect(clippy::let_unit_value))]
        let _message_id =
            api::echo(chunks.to_iter(include_title), add_to_history, opts)
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
                NvimEcho::set_cmdheight(min_cmdheight);
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
                ProgressNotificationKind::Progress(_) => {},
            }

            'spin: loop {
                select_biased! {
                    _ = spin.next().fuse() => {
                        spinner_frame_idx += 1;
                        spinner_frame_idx %= SPINNER_FRAMES.len();
                        chunks.title.icon = Icon::Char(SPINNER_FRAMES[spinner_frame_idx]);
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
}

impl ProgressNotificationKind {
    fn hl_group(self) -> &'static str {
        match self {
            Self::Progress(_) => "DiagnosticInfo",
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

    fn to_iter(
        &self,
        include_title: bool,
    ) -> impl Iterator<Item = (nvim_oxi::String, Option<&str>)> {
        let title = include_title
            .then(|| (self.title.to_compact_string(), self.title.hl_group));

        title
            .into_iter()
            .chain(iter::once((CompactString::const_new("\n"), None::<&str>)))
            .chain(self.message_chunks.iter().map(|chunk| {
                (chunk.text_as_compact_str().clone(), chunk.hl_group())
            }))
            .map(|(text, hl_group)| (text.as_str().into(), hl_group))
    }
}

impl fmt::Display for Title<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // On Nightly the percentage is set in the opts, so we don't need to
        // include it in the title.
        if matches!(self.icon, Icon::Percentage(_))
            && cfg!(feature = "nightly")
        {
            return self.namespace.dot_separated().fmt(f);
        }

        write!(f, "{} {}", self.icon, self.namespace.dot_separated())
    }
}
