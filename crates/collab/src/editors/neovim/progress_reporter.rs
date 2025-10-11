use core::time::Duration;

use abs_path::NodeNameBuf;
use editor::notify::{self, Emitter};
use editor::{Context, Editor};
use flume::TrySendError;
use futures_util::{FutureExt, StreamExt, select_biased};
use neovim::Neovim;

use crate::config;
use crate::progress::{JoinState, ProgressReporter, StartState};

/// Frames for the spinner animation.
const SPINNER_FRAMES: &[&str] = &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];

/// How many revolutions per minute the spinner should complete.
const SPINNER_RPM: u8 = 75;

/// How often the spinner should be updated to achieve the desired RPM.
const SPINNER_UPDATE_INTERVAL: Duration = Duration::from_millis({
    (60_000.0 / ((SPINNER_RPM as u16 * SPINNER_FRAMES.len() as u16) as f32))
        .round() as u64
});

pub struct NeovimProgressReporter {
    message_tx: flume::Sender<Message>,
    project_name: Option<NodeNameBuf>,
    server_address: Option<config::ServerAddress<'static>>,
}

struct Message {
    level: notify::Level,
    text: String,
    is_last: bool,
}

impl ProgressReporter<Neovim> for NeovimProgressReporter {
    fn new(ctx: &mut Context<Neovim>) -> Self {
        let (message_tx, message_rx) = flume::bounded::<Message>(4);

        ctx.spawn_local(async move |ctx| {
            let namespace = ctx.namespace().clone();
            let mut spin = async_io::Timer::interval(SPINNER_UPDATE_INTERVAL);
            let mut messages = message_rx.into_stream();

            let Some(mut message) = messages.next().await else { return };
            let mut spinner_frame_idx = 0;
            let mut prev_id = None;

            loop {
                prev_id = ctx.with_editor(|nvim| {
                    Some(nvim.emitter().emit(notify::Notification {
                        level: message.level,
                        message: notify::Message::from_display(format_args!(
                            "{} {}",
                            SPINNER_FRAMES[spinner_frame_idx], message.text,
                        )),
                        namespace: &namespace,
                        updates_prev: prev_id,
                    }))
                });

                if message.is_last {
                    break;
                }

                select_biased! {
                    _ = spin.next().fuse() => {
                        spinner_frame_idx += 1;
                        spinner_frame_idx %= SPINNER_FRAMES.len();
                    },
                    next_message = messages.select_next_some() => {
                        message = next_message
                    },
                }
            }
        })
        .detach();

        Self { message_tx, project_name: None, server_address: None }
    }

    fn report_join_progress(
        &mut self,
        mut state: JoinState<'_>,
        _: &mut Context<Neovim>,
    ) {
        let text = loop {
            match &state {
                JoinState::ConnectingToServer { server_addr } => {
                    self.server_address = Some(server_addr.to_owned());
                    state = JoinState::JoiningSession;
                },

                JoinState::JoiningSession => {
                    let server_addr = self.server_address.as_ref().expect(
                        "StartingSession must be preceded by \
                         ConnectingToServer",
                    );
                    break format!("Connecting to server at {server_addr}");
                },

                JoinState::ReceivingProject { project_name } => {
                    self.project_name = Some((**project_name).to_owned());
                    break format!("Receiving files for {project_name}");
                },

                JoinState::WritingProject { root_path } => {
                    let project_name = self.project_name.as_ref().expect(
                        "WritingProject must be preceded by ReceivingProject",
                    );
                    break format!("Writing {project_name} to {root_path}");
                },

                JoinState::Done => break "Joined session".to_owned(),
            }
        };

        let message = Message {
            level: notify::Level::Info,
            text: text.to_owned(),
            is_last: matches!(state, JoinState::Done),
        };

        if let Err(err) = self.message_tx.try_send(message) {
            match err {
                TrySendError::Disconnected(_) => unreachable!(),
                TrySendError::Full(_) => {},
            }
        }
    }

    fn report_start_progress(
        &mut self,
        mut state: StartState<'_>,
        _: &mut Context<Neovim>,
    ) {
        let text = loop {
            match &state {
                StartState::ConnectingToServer { server_addr } => {
                    self.server_address = Some(server_addr.to_owned());
                    state = StartState::StartingSession;
                },
                StartState::StartingSession => {
                    let server_addr = self.server_address.as_ref().expect(
                        "StartingSession must be preceded by \
                         ConnectingToServer",
                    );
                    break format!("Connecting to server at {server_addr}");
                },
                StartState::ReadingProject { root_path } => {
                    break format!("Reading project at {root_path}");
                },
                StartState::Done => break "Started session".to_owned(),
            }
        };

        let message = Message {
            level: notify::Level::Info,
            text: text.to_owned(),
            is_last: matches!(state, StartState::Done),
        };

        if let Err(err) = self.message_tx.try_send(message) {
            match err {
                TrySendError::Disconnected(_) => unreachable!(),
                TrySendError::Full(_) => {},
            }
        }
    }
}

impl Drop for NeovimProgressReporter {
    fn drop(&mut self) {
        let _ = self.message_tx.send(Message {
            level: notify::Level::Info,
            text: "".to_owned(),
            is_last: true,
        });
    }
}
