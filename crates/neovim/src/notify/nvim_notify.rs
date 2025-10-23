use core::ops::Range;
use core::time::Duration;
use core::{any, fmt};

use compact_str::ToCompactString;
use executor::LocalSpawner;
use flume::TrySendError;
use futures_util::{FutureExt, StreamExt, select_biased};
use nvim_oxi::{api, mlua};

use crate::buffer::Point;
use crate::executor::NeovimLocalSpawner;
use crate::notify::progress_reporter::{
    ProgressNotification,
    ProgressNotificationKind,
};
use crate::{notify, utils};

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
    notif_tx: flume::Sender<ProgressNotification>,
}

pub(super) enum Icon {
    Char(char),
    Percentage(notify::Percentage),
}

struct HlRanges<'chunks, Lines: Iterator> {
    lines: Lines,
    current_line: Option<Lines::Item>,
    message_chunks: &'chunks notify::Chunks,
}

impl NvimNotify {
    pub(crate) fn notify(
        namespace: &editor::notify::Namespace,
        message_chunks: notify::Chunks,
        level: notify::Level,
        namespace_id: u32,
    ) {
        let lua = mlua::lua();

        let notify = notify(&lua);

        let opts = lua
            .create_table_with_capacity(0, 2)
            .expect("failed to create options table");

        opts.raw_set("title", namespace.dot_separated().to_string())
            .expect("failed to set 'title'");

        opts.raw_set(
            "on_open",
            Self::on_open(message_chunks.clone(), namespace_id),
        )
        .expect("failed to set 'on_open'");

        notify
            .call::<mlua::Value>((
                &*message_chunks.concat_text(),
                level as u8,
                &opts,
            ))
            .expect("failed to call 'notify'");
    }

    #[inline]
    pub(super) fn is_installed() -> bool {
        utils::is_module_available("notify")
    }

    fn on_open(
        message_chunks: notify::Chunks,
        namespace_id: u32,
    ) -> mlua::Function {
        mlua::lua()
            .create_function(move |_, window: api::Window| {
                let Ok(mut buf) = window.get_buf() else { return Ok(()) };
                let Ok(lines) = buf
                    .line_count()
                    .and_then(|count| buf.get_lines(0..count, true))
                else {
                    return Ok(());
                };
                let hl_ranges = HlRanges::new(lines, &message_chunks);
                for (hl_group, point_range) in hl_ranges {
                    let _ = buf.set_extmark(
                        namespace_id,
                        point_range.start.newline_offset,
                        point_range.end.byte_offset,
                        &api::opts::SetExtmarkOpts::builder()
                            .end_row(point_range.start.newline_offset)
                            .end_col(point_range.end.byte_offset)
                            .hl_group(hl_group)
                            .build(),
                    );
                }
                Ok(())
            })
            .expect("couldn't create function")
    }
}

impl NvimNotifyProgressReporter {
    /// Creates a new progress reporter.
    pub fn new(
        namespace: editor::notify::Namespace,
        spawner: &mut NeovimLocalSpawner,
    ) -> Self {
        let (notif_tx, notif_rx) = flume::unbounded();

        spawner
            .spawn(async move {
                Self::event_loop(notif_rx, &namespace, &mlua::lua()).await;
            })
            .detach();

        Self { notif_tx }
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
        if let Err(err) = self.notif_tx.try_send(notif) {
            match err {
                TrySendError::Disconnected(_) => tracing::error!(
                    "{}'s event loop panicked",
                    any::type_name::<Self>()
                ),
                TrySendError::Full(_) => unreachable!("channel is unbounded"),
            }
        }
    }

    async fn event_loop(
        notif_rx: flume::Receiver<ProgressNotification>,
        namespace: &editor::notify::Namespace,
        mlua: &mlua::Lua,
    ) {
        let Ok(first_notif) = notif_rx.recv_async().await else { return };

        let mut spin = async_io::Timer::interval(SPINNER_UPDATE_INTERVAL);
        let mut spinner_frame_idx = 0;

        let notify = notify(mlua);

        let opts = mlua
            .create_table_with_capacity(0, 4)
            .expect("failed to create options table");

        opts.raw_set("title", namespace.dot_separated().to_string())
            .expect("failed to set 'title'");

        let mut notif = first_notif;

        loop {
            let hide_from_history =
                notif.kind != ProgressNotificationKind::Error;

            opts.raw_set("hide_from_history", hide_from_history)
                .expect("failed to set 'hide_from_history'");

            opts.raw_set(
                "icon",
                &*notif.kind.icon(spinner_frame_idx).to_compact_string(),
            )
            .expect("failed to set 'icon'");

            Self::notify(&notif, &opts, &notify);

            if !matches!(notif.kind, ProgressNotificationKind::Progress(_)) {
                break;
            }

            'spin: loop {
                select_biased! {
                    _ = spin.next().fuse() => {
                        spinner_frame_idx += 1;
                        spinner_frame_idx %= SPINNER_FRAMES.len();
                        let frame = SPINNER_FRAMES[spinner_frame_idx];
                        opts.raw_set("icon", frame).expect("failed to set 'icon'");
                        Self::notify(&notif, &opts, &notify);
                        continue 'spin;
                    },

                    maybe_notif = notif_rx.recv_async() => {
                        let Ok(new_notif) = maybe_notif else { return; };

                        match (notif.kind, new_notif.kind) {
                            // Stop spinning if we've started showing
                            // percentages.
                            (
                                ProgressNotificationKind::Progress(None),
                                ProgressNotificationKind::Progress(Some(_)),
                            ) => spin.clear(),

                            // Start spinning if we've stopped showing
                            // percentages.
                            (
                                ProgressNotificationKind::Progress(Some(_)),
                                ProgressNotificationKind::Progress(None),
                            ) => spin.set_interval(SPINNER_UPDATE_INTERVAL),

                            _ => {},
                        }

                        notif = new_notif;
                        break 'spin;
                    },
                }
            }
        }
    }

    fn notify(
        notif: &ProgressNotification,
        opts: &mlua::Table,
        notify: &mlua::Function,
    ) {
        let record = notify
            .call::<mlua::Table>((
                notif.chunks.concat_text(),
                notify::Level::from(notif.kind) as u8,
                opts,
            ))
            .expect("failed to call 'notify'");

        let new_id = record
            .get::<mlua::Integer>("id")
            .expect("failed to get notification ID from record");

        opts.raw_set("replace", new_id).expect("failed to set 'replace'");
    }
}

impl ProgressNotificationKind {
    pub(super) fn icon(self, spinner_frame_idx: usize) -> Icon {
        let char = match self {
            Self::Progress(Some(perc)) => return Icon::Percentage(perc),
            Self::Progress(None) => SPINNER_FRAMES[spinner_frame_idx],
            Self::Success => '✔',
            Self::Error => '✘',
        };
        Icon::Char(char)
    }
}

impl<'chunks, Lines: Iterator> HlRanges<'chunks, Lines> {
    fn new(lines: Lines, message_chunks: &'chunks notify::Chunks) -> Self {
        HlRanges { lines, current_line: None, message_chunks }
    }
}

#[cfg(feature = "nightly")]
impl From<ProgressNotificationKind>
    for nvim_oxi::api::types::ProgressMessageStatus
{
    fn from(kind: ProgressNotificationKind) -> Self {
        match kind {
            ProgressNotificationKind::Progress(_) => Self::Running,
            ProgressNotificationKind::Success => Self::Success,
            ProgressNotificationKind::Error => Self::Failed,
        }
    }
}

impl fmt::Display for Icon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Char(c) => write!(f, "{}", c),
            Self::Percentage(perc) => write!(f, "{perc}%"),
        }
    }
}

impl<'chunks, Lines: Iterator<Item = nvim_oxi::String>> Iterator
    for HlRanges<'chunks, Lines>
{
    type Item = (&'chunks str, Range<Point>);

    fn next(&mut self) -> Option<Self::Item> {
        None
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
