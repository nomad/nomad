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
use crate::notify::Chunk;
use crate::notify::progress_reporter::{ProgressNotification, ProgressStatus};
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

struct HlRanges<Lines: Iterator, Chunks> {
    lines: Lines,
    current_line: Option<Lines::Item>,
    message_chunks: Chunks,
    highlight_end: Point,
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
            .expect("couldn't create options table");

        opts.raw_set("title", namespace.dot_separated().to_string())
            .expect("couldn't set 'title'");

        opts.raw_set(
            "on_open",
            Self::on_open(message_chunks.clone(), namespace_id),
        )
        .expect("couldn't set 'on_open'");

        notify
            .call::<mlua::Value>((
                &*message_chunks.concat_text(),
                level as u8,
                &opts,
            ))
            .expect("couldn't call 'notify'");
    }

    #[inline]
    pub(super) fn is_installed() -> bool {
        utils::is_module_available("notify")
    }

    fn on_open(
        message_chunks: notify::Chunks,
        namespace_id: u32,
    ) -> mlua::Function {
        fn inner(
            window: api::Window,
            message_chunks: notify::Chunks,
            namespace_id: u32,
        ) -> Result<(), api::Error> {
            let mut buf = window.get_buf()?;
            let line_count = buf.line_count()?;
            let lines = buf.get_lines(0..line_count, true)?;
            let hl_ranges = HlRanges::new(lines, message_chunks.iter());
            for (hl_group, point_range) in hl_ranges {
                buf.set_extmark(
                    namespace_id,
                    point_range.start.newline_offset,
                    point_range.start.byte_offset,
                    &api::opts::SetExtmarkOpts::builder()
                        .end_row(point_range.end.newline_offset)
                        .end_col(point_range.end.byte_offset)
                        .hl_group(hl_group)
                        .build(),
                )?;
            }
            Ok(())
        }

        mlua::lua()
            .create_function(move |_, window: api::Window| {
                if let Err(err) =
                    inner(window, message_chunks.clone(), namespace_id)
                {
                    tracing::error!(
                        "couldn't apply highlights in nvim-notify \
                         notification: {err:?}"
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
            status: ProgressStatus::Error,
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
            status: ProgressStatus::Progress(perc),
        });
    }

    /// TODO: docs.
    pub fn report_success(self, chunks: notify::Chunks) {
        self.send_notification(ProgressNotification {
            chunks,
            status: ProgressStatus::Success,
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
            .create_table_with_capacity(0, 5)
            .expect("couldn't create options table");

        opts.raw_set("title", namespace.dot_separated().to_string())
            .expect("couldn't set 'title'");

        let mut notif = first_notif;

        loop {
            let hide_from_history = notif.status != ProgressStatus::Error;

            opts.raw_set("hide_from_history", hide_from_history)
                .expect("couldn't set 'hide_from_history'");

            opts.raw_set(
                "icon",
                &*notif.status.icon(spinner_frame_idx).to_compact_string(),
            )
            .expect("couldn't set 'icon'");

            Self::notify(&notif, &opts, &notify);

            if !matches!(notif.status, ProgressStatus::Progress(_)) {
                break;
            }

            'spin: loop {
                select_biased! {
                    _ = spin.next().fuse() => {
                        spinner_frame_idx += 1;
                        spinner_frame_idx %= SPINNER_FRAMES.len();
                        let frame = SPINNER_FRAMES[spinner_frame_idx];
                        opts.raw_set("icon", frame).expect("couldn't set 'icon'");
                        Self::notify(&notif, &opts, &notify);
                        continue 'spin;
                    },

                    maybe_notif = notif_rx.recv_async() => {
                        let Ok(new_notif) = maybe_notif else { return; };

                        match (notif.status, new_notif.status) {
                            // Stop spinning if we've started showing
                            // percentages.
                            (
                                ProgressStatus::Progress(None),
                                ProgressStatus::Progress(Some(_)),
                            ) => spin.clear(),

                            // Start spinning if we've stopped showing
                            // percentages.
                            (
                                ProgressStatus::Progress(Some(_)),
                                ProgressStatus::Progress(None),
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
        let timeout = match notif.status {
            // Keep the notification window open for as long as we're emitting
            // progress messages. Without this, the window would be closed if
            // the time between two consecutive progress updates is greater
            // than the 'timeout' value configured by the user.
            ProgressStatus::Progress(_) => mlua::Value::Boolean(false),
            // Re-enable the timeout when we emit the final progress message.
            ProgressStatus::Success => mlua::Value::Integer(2500),
            ProgressStatus::Error => mlua::Value::Integer(3500),
        };

        opts.raw_set("timeout", timeout).expect("couldn't set 'timeout'");

        let record = notify
            .call::<mlua::Table>((
                notif.chunks.concat_text(),
                notify::Level::from(notif.status) as u8,
                opts,
            ))
            .expect("couldn't call 'notify'");

        let new_id = record
            .get::<mlua::Integer>("id")
            .expect("couldn't get notification ID from record");

        opts.raw_set("replace", new_id).expect("couldn't set 'replace'");
    }
}

impl ProgressStatus {
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

impl<Lines, Chunks> HlRanges<Lines, Chunks>
where
    Lines: ExactSizeIterator<Item = nvim_oxi::String> + DoubleEndedIterator,
{
    fn new(mut lines: Lines, message_chunks: Chunks) -> Self {
        let last_line = lines.next_back();
        let highlight_end = Point {
            newline_offset: lines.len(),
            byte_offset: last_line.as_ref().map_or(0, |line| line.len()),
        };
        Self { lines, current_line: last_line, highlight_end, message_chunks }
    }
}

#[cfg(feature = "nightly")]
impl From<ProgressStatus> for nvim_oxi::api::types::ProgressMessageStatus {
    fn from(status: ProgressStatus) -> Self {
        match status {
            ProgressStatus::Progress(_) => Self::Running,
            ProgressStatus::Success => Self::Success,
            ProgressStatus::Error => Self::Failed,
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

impl<'chunks, Lines, Chunks> Iterator for HlRanges<Lines, Chunks>
where
    Lines: DoubleEndedIterator<Item = nvim_oxi::String>,
    Chunks: DoubleEndedIterator<Item = &'chunks Chunk>,
{
    type Item = (&'chunks str, Range<Point>);

    fn next(&mut self) -> Option<Self::Item> {
        let (text, hl_group) = loop {
            let chunk = self.message_chunks.next_back()?;
            let Some(hl_group) = chunk.hl_group() else { continue };
            break (chunk.text(), hl_group);
        };

        loop {
            let full_line = self.current_line.as_ref()?.as_bytes();

            let haystack = &full_line[..self.highlight_end.byte_offset];

            let start_byte_offset =
                match memchr::memmem::rfind(haystack, text.as_bytes()) {
                    Some(offset) => offset,
                    None => {
                        let prev_line = self.lines.next_back()?;
                        self.highlight_end.newline_offset -= 1;
                        self.highlight_end.byte_offset = prev_line.len();
                        self.current_line = Some(prev_line);
                        continue;
                    },
                };

            let hl_start = Point {
                newline_offset: self.highlight_end.newline_offset,
                byte_offset: start_byte_offset,
            };

            let hl_end = Point {
                newline_offset: hl_start.newline_offset,
                byte_offset: hl_start.byte_offset + text.len(),
            };

            self.highlight_end.byte_offset = start_byte_offset;

            return Some((hl_group, hl_start..hl_end));
        }
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
        .expect("couldn't require 'notify' module");

    nvim_notify
        .get::<mlua::Function>("notify")
        .expect("'notify' function not found in 'notify' module")
}
